pub mod ssl;
pub mod tcp;

use crate::{
    bosd::{
        BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializable, BorrowingSerializer,
        OwningDeserializble, OwningDeserializer, OwningSerializable, OwningSerializer,
    },
    msg::{
        bin_bytes_buf::BorrowingStrBuf,
        header::{MsgType, OwningStandardHeader},
        version::BorrowingVersion,
    },
};

use self::ssl::IrodsSSLSettings;
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use std::{
    io::{self, Cursor, Write},
    marker::PhantomData,
    time::Duration,
};

use base64::engine::GeneralPurpose;
use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};

static MAX_PASSWORD_LEN: usize = 50;

pub enum CsNegPolicy {
    DontCare,
    Require,
    Refuse,
}

pub struct ConnConfig<S>
where
    S: io::Read + io::Write,
{
    pub buf_size: usize,
    pub request_timeout: Duration,
    pub read_timeout: Duration,
    pub a_ttl: u32,
    cs_neg_policy: CsNegPolicy,
    ssl_config: Option<IrodsSSLSettings>,
    pub addr: (String, u16),
    phantom_transport: PhantomData<S>,
}

pub struct Account {
    pub auth_scheme: AuthenticationScheme,
    pub client_user: String,
    pub client_zone: String,
    pub proxy_user: String,
    pub proxy_zone: String,
    pub password: String,
}

pub enum AuthenticationScheme {
    Native,
}

pub struct Connection<T, S>
where
    T: BorrowingSerializer + BorrowingDeserializer + OwningSerializer + OwningDeserializer,
    S: io::Read + io::Write,
{
    account: Account,
    config: ConnConfig<S>,
    buf: Vec<u8>,
    socket: S,
    // FIXME: Make this a statically sized array
    signature: Vec<u8>,
    phantom_protocol: PhantomData<T>,
}

impl<T, S> Connection<T, S>
where
    T: BorrowingSerializer + BorrowingDeserializer + OwningSerializer + OwningDeserializer,
    S: io::Read + io::Write,
{
    pub fn new(account: Account, config: ConnConfig<S>, socket: S) -> Self {
        let buf = vec![0; config.buf_size];
        let signature = vec![0; 16];
        Connection {
            account,
            config,
            buf,
            socket,
            signature,
            phantom_protocol: PhantomData,
        }
    }

    /// This method does things by hand, which is very annoying,
    /// but it's the only way to get the job done in an RAII manner
    fn authenticate(
        account: &Account,
        config: &ConnConfig<S>,
        socket: &mut S,
        header_buf: &mut Vec<u8>,
        msg_buf: &mut Vec<u8>,
    ) -> Result<Vec<u8>, IrodsError> {
        let b64_engine = Self::create_base64_engine();

        // UNSAFE: Connection buffers are always initialized with
        // at least enough space for the payload
        let mut header_cursor = Cursor::new(header_buf);
        let mut msg_cursor = Cursor::new(msg_buf);
        let mut tmp_buf: [u8; 4] = [0; 4];

        write!(
            header_cursor,
            r##"
        {{
            "a_ttl":"{0}",
            "force_password_prompt": "true",
            "next_operation": "auth_agent_auth_request",
            "scheme": "native",
            "user_name": "{1}",
            "zone_name": "{2}"
        }}
        "##,
            config.a_ttl, account.client_user, account.client_zone
        )?;

        let payload_len = b64_engine
            .encode_slice(
                &header_buf[..header_cursor.position() as usize],
                msg_buf.as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks.".into()))?;

        // UNSAFE: Base64 is always valid UTF-8
        let str_buf =
            BorrowingStrBuf::new(unsafe { std::str::from_utf8_unchecked(&msg_buf[..payload_len]) });

        let msg_len = T::rods_borrowing_ser(&str_buf, msg_buf)?;

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, header_buf)?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        socket.write_all(&(header_len as u32).to_be_bytes())?;
        socket.write_all(&header_buf[..header_len])?;
        socket.write_all(&msg_buf[..msg_len])?;

        // Receive server reply.
        socket.read_exact(tmp_buf.as_mut())?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(Self::read_from_server_uninit(
            header_len, header_buf, socket,
        )?)?;

        let msg: BorrowingStrBuf = T::rods_borrowing_de(Self::read_from_server_uninit(
            header.msg_len,
            msg_buf,
            socket,
        )?)?;

        let mut digest = Md5::new();
        digest.update(msg.buf.as_bytes());

        let mut pad_buf = &mut header_buf[..MAX_PASSWORD_LEN];
        pad_buf.fill(0);
        for (i, c) in account.password.as_bytes().iter().enumerate() {
            pad_buf[i] = *c;
            i += 1;
        }
        digest.update(pad_buf);

        let header_cursor = Cursor::new(header_buf);
        let msg_cursor = Cursor::new(msg_buf);
        write!(
            header_cursor,
            r#"
        {{
            "a_ttl": {0},
            "force_password_prompt": "true",
            "next_operation": "auth_agent_auth_response",
            "request_result": "{1}",
            "scheme": "native",
            "user_name": "{2}",
            "zone_name": "{3}",
            "digest": "{4}"
        }}"#,
            config.a_ttl,
            msg.buf,
            account.client_user,
            account.client_zone,
            STANDARD.encode(digest.finalize())
        );

        let payload_len = b64_engine
            .encode_slice(
                &header_buf[..header_cursor.position() as usize],
                msg_buf.as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks".into()))?;

        let str_buf =
            BorrowingStrBuf::new(unsafe { std::str::from_utf8_unchecked(&msg_buf[..payload_len]) });
        let msg_len = T::rods_borrowing_ser(&str_buf, msg_buf)?;

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, header_buf)?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        socket.write_all(&(header_len as u32).to_be_bytes())?;
        socket.write_all(&header_buf[..header_len])?;
        socket.write_all(&msg_buf[..msg_len])?;

        // Receive server reply.
        
        socket.read_exact(tmp_buf.as_mut())?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(Self::read_from_server_uninit(
            header_len, header_buf, socket,
        )?)?;

        let msg: BorrowingStrBuf = T::rods_borrowing_de(Self::read_from_server_uninit(
            header.msg_len,
            msg_buf,
            socket,
        )?)?;


        Ok(())
    }

    /// Private function to create a base64 engine from
    /// a config that allows decode trailing bits and a standard alphabet
    fn create_base64_engine() -> GeneralPurpose {
        let cfg = GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true);
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }

    fn read_from_server_uninit<'s, 'r>(
        len: usize,
        buf: &'s mut Vec<u8>,
        socket: &'s mut S,
    ) -> Result<&'r [u8], IrodsError>
    where
        's: 'r,
    {
        if len > buf.len() {
            buf.resize(len, 0);
        }
        socket.read_exact(&mut buf[..len])?;
        Ok(&buf[..len])
    }

    fn read_from_server(&mut self, len: usize) -> Result<&[u8], IrodsError> {
        if len > self.buf.len() {
            self.buf.resize(len, 0);
        }
        self.socket.read_exact(&mut self.buf[..len])?;
        Ok(&self.buf[..len])
    }

    fn push_owning(&mut self, msg: &impl OwningSerializable) -> Result<usize, IrodsError> {
        let msg_len = T::rods_owning_ser(msg, &mut self.buf)?;
        self.socket.write(&mut self.buf[..msg_len])?;
        Ok(msg_len)
    }

    fn push_borrowing<'s, 'r>(
        &'r mut self,
        msg: &'s impl BorrowingSerializable<'s>,
    ) -> Result<usize, IrodsError>
    where
        's: 'r,
    {
        let msg_len = T::rods_borrowing_ser(msg, &mut self.buf)?;
        self.socket.write_all(&mut self.buf[..msg_len])?;
        Ok(msg_len)
    }

    // This behavior is slightly different since you
    // don't need to pass length as an argument
    pub fn pull_header(&mut self) -> Result<OwningStandardHeader, IrodsError> {
        let header_size =
            u32::from_be_bytes(self.read_from_server(4)?.try_into().unwrap()) as usize;
        self.pull_owning(header_size)
    }

    pub fn pull_owning<M>(&mut self, len: usize) -> Result<M, IrodsError>
    where
        M: OwningDeserializble,
    {
        T::rods_owning_de(self.read_from_server(len)?)
    }

    pub fn pull_borrowing<'s, 'r, M>(&'s mut self, len: usize) -> Result<M, IrodsError>
    where
        M: BorrowingDeserializable<'r>,
        's: 'r,
    {
        T::rods_borrowing_de(self.read_from_server(len)?)
    }
}
