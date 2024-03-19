#![allow(warnings)]

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

#[derive(Clone)]
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

// Manually implement Clone for ConnConfig because it contains a phantom data field
// which is not Clone 
impl<S> Clone for ConnConfig<S>
where
    S: io::Read + io::Write,
{
    fn clone(&self) -> Self {
        ConnConfig {
            buf_size: self.buf_size,
            request_timeout: self.request_timeout,
            read_timeout: self.read_timeout,
            a_ttl: self.a_ttl,
            cs_neg_policy: self.cs_neg_policy.clone(),
            ssl_config: self.ssl_config.clone(),
            addr: self.addr.clone(),
            phantom_transport: PhantomData,
        }
    }
}

#[derive(Clone)]
pub struct Account {
    pub auth_scheme: AuthenticationScheme,
    pub client_user: String,
    pub client_zone: String,
    pub proxy_user: String,
    pub proxy_zone: String,
    pub password: String,
}

#[cfg(test)]
impl Account {
    fn test_account() -> Account {
        Account {
            auth_scheme: AuthenticationScheme::Native,
            client_user: "rods".into(),
            client_zone: "tempZone".into(),
            proxy_user: "rods".into(),
            proxy_zone: "tempZone".into(),
            password: "rods".into(),
        }
    }
}

#[derive(Clone)]
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
    header_buf: Vec<u8>,
    msg_buf: Vec<u8>,
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
        let mut tmp_buf = [0u8; 4];

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

        let unencoded_len = header_cursor.position() as usize;
        let payload_len = b64_engine
            .encode_slice(
                &header_cursor.get_mut()[..unencoded_len],
                msg_cursor.get_mut().as_mut_slice()
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks.".into()))?;

        // UNSAFE: Base64 is always valid UTF-8
        let encoded_str = unsafe { std::str::from_utf8_unchecked(&msg_cursor.get_ref()[..payload_len]) };
        let str_buf =
            BorrowingStrBuf::new(encoded_str);

        // We're being very naughty here and serializing the msg into the
        // thing called "header" buf. This unfortunate, but I can't think
        // of a better way to do it right now that gets around the 
        // borrow checker.
        let msg_len = T::rods_borrowing_ser(&str_buf, header_cursor.get_mut())?;

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, msg_cursor.get_mut())?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        socket.write_all(&(header_len as u32).to_be_bytes())?;
        socket.write_all(&msg_cursor.get_ref()[..msg_len])?;
        socket.write_all(&header_cursor.get_ref()[..header_len])?;

        // Receive server reply.
        socket.read_exact(tmp_buf.as_mut())?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(Self::read_from_server_uninit(
            header_len, header_cursor.get_mut(), socket,
        )?)?;

        // After this point, there should be no extent borrows of the buffers

        header_cursor.set_position(0);
        msg_cursor.set_position(0);

        let msg: BorrowingStrBuf = T::rods_borrowing_de(Self::read_from_server_uninit(
            header.msg_len,
            msg_cursor.get_mut(),
            socket,
        )?)?;

        let mut digest = Md5::new();
        digest.update(msg.buf.as_bytes());

        let mut pad_buf = &mut header_cursor.get_mut()[..MAX_PASSWORD_LEN];
        pad_buf.fill(0);
        for (i, c) in account.password.as_bytes().iter().enumerate() {
            pad_buf[i] = *c;
        }
        digest.update(pad_buf);


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

        let unencoded_len = header_cursor.position() as usize;
        let payload_len = b64_engine
            .encode_slice(
                &header_cursor.get_mut()[..unencoded_len],
                msg_cursor.get_mut().as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks".into()))?;

        let encoded_str = unsafe { std::str::from_utf8_unchecked(&msg_cursor.get_ref()[..payload_len]) };
        let str_buf =
            BorrowingStrBuf::new(encoded_str);
        let msg_len = T::rods_borrowing_ser(&str_buf, header_cursor.get_mut())?;

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, msg_cursor.get_mut())?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        socket.write_all(&(header_len as u32).to_be_bytes())?;
        socket.write_all(&msg_cursor.get_mut()[..msg_len])?;
        socket.write_all(&header_cursor.get_mut()[..header_len])?;

        // Receive server reply.
        header_cursor.set_position(0);
        msg_cursor.set_position(0);
        
        socket.read_exact(tmp_buf.as_mut())?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(Self::read_from_server_uninit(
            header_len, header_cursor.get_mut(), socket,
        )?)?;

        let msg: BorrowingStrBuf = T::rods_borrowing_de(Self::read_from_server_uninit(
            header.msg_len,
            msg_cursor.get_mut(),
            socket,
        )?)?;


        Ok(Vec::new())
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

    fn read_into_msg_buf(&mut self, len: usize) -> Result<&[u8], IrodsError> {
        if len > self.msg_buf.len() {
            self.msg_buf.resize(len, 0);
        }
        self.socket.read_exact(&mut self.msg_buf[..len])?;
        Ok(&self.msg_buf[..len])
    }

    fn read_into_header_buf(&mut self, len: usize) -> Result<&[u8], IrodsError> {
        if len > self.header_buf.len() {
            self.header_buf.resize(len, 0);
        }
        self.socket.read_exact(&mut self.header_buf[..len])?;
        Ok(&self.header_buf[..len])
    }

    fn push_owning_from_msg_buf(&mut self, msg: &impl OwningSerializable) -> Result<usize, IrodsError> {
        let msg_len = T::rods_owning_ser(msg, &mut self.msg_buf)?;
        self.socket.write(&mut self.msg_buf[..msg_len])?;
        Ok(msg_len)
    }

    fn push_owning_from_header_buf(&mut self, msg: &impl OwningSerializable) -> Result<usize, IrodsError> {
        let msg_len = T::rods_owning_ser(msg, &mut self.header_buf)?;
        self.socket.write(&mut self.header_buf[..msg_len])?;
        Ok(msg_len)
    }

    fn push_borrowing_from_msg_buf<'s, 'r>(
        &'r mut self,
        msg: &'s impl BorrowingSerializable<'s>,
    ) -> Result<usize, IrodsError>
    where
        's: 'r,
    {
        let msg_len = T::rods_borrowing_ser(msg, &mut self.msg_buf)?;
        self.socket.write_all(&mut self.msg_buf[..msg_len])?;
        Ok(msg_len)
    }

    fn push_borrowing_from_header_buf<'s, 'r>(
        &'r mut self,
        msg: &'s impl BorrowingSerializable<'s>,
    ) -> Result<usize, IrodsError>
    where
        's: 'r,
    {
        let msg_len = T::rods_borrowing_ser(msg, &mut self.header_buf)?;
        self.socket.write_all(&mut self.header_buf[..msg_len])?;
        Ok(msg_len)
    }

    // This behavior is slightly different since you
    // don't need to pass length as an argument
    pub fn pull_header(&mut self) -> Result<OwningStandardHeader, IrodsError> {
        let header_size =
            u32::from_be_bytes(self.read_into_header_buf(4)?.try_into().unwrap()) as usize;
        self.pull_owning_into_header_buf(header_size)
    }

    pub fn pull_owning_into_msg_buf<M>(&mut self, len: usize) -> Result<M, IrodsError>
    where
        M: OwningDeserializble,
    {
        T::rods_owning_de(self.read_into_msg_buf(len)?)
    }

    fn pull_owning_into_header_buf<M>(&mut self, len: usize) -> Result<M, IrodsError>
    where
        M: OwningDeserializble,
    {
        T::rods_owning_de(self.read_into_header_buf(len)?)
    }

    pub fn pull_borrowing_into_msg_buf<'s, 'r, M>(&'s mut self, len: usize) -> Result<M, IrodsError>
    where
        M: BorrowingDeserializable<'r>,
        's: 'r,
    {
        T::rods_borrowing_de(self.read_into_msg_buf(len)?)
    }

    pub fn pull_borrowing_into_header_buf<'s, 'r, M>(&'s mut self, len: usize) -> Result<M, IrodsError>
    where
        M: BorrowingDeserializable<'r>,
        's: 'r,
    {
        T::rods_borrowing_de(self.read_into_header_buf(len)?)
    }
}
