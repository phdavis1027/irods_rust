pub mod ssl;
pub mod tcp;

use std::{io, marker::PhantomData, time::Duration};

use crate::{
    bosd::{
        BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializer, OwningDeserializble,
        OwningDeserializer, OwningSerializable, OwningSerializer,
    },
    msg::header::OwningStandardHeader,
};

use self::ssl::IrodsSSLSettings;

use base64::engine::{GeneralPurpose, GeneralPurposeConfig};
use rods_prot_msg::error::errors::IrodsError;

pub enum CsNegPolicy {
    DontCare,
    Require,
    Refuse,
}

pub struct ConnConfig<T>
where
    T: BorrowingSerializer + BorrowingDeserializer + OwningSerializer + OwningDeserializer,
{
    pub buf_size: usize,
    pub request_timeout: Duration,
    pub read_timeout: Duration,
    pub a_ttl: u32,
    cs_neg_policy: CsNegPolicy,
    ssl_config: Option<IrodsSSLSettings>,
    pub addr: (String, u16),
    phantom_transport: PhantomData<T>,
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
    config: ConnConfig<T>,
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
    pub fn new(account: Account, config: ConnConfig<T>, socket: S) -> Self {
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

    /// Private function to create a base64 engine from
    /// a config that allows decode trailing bits and a standard alphabet
    fn create_base64_engine() -> GeneralPurpose {
        let cfg = GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true);
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }

    fn read_from_server(&mut self, len: usize) -> Result<&[u8], IrodsError> {
        if len > self.buf.len() {
            self.buf.resize(len, 0);
        }
        self.socket.read_exact(&mut self.buf[..len])?;
        Ok(&self.buf[..len])
    }

    fn write_to_buf(&mut self, payload: &[u8]) -> Result<(), IrodsError> {
        if payload.len() > self.buf.len() {
            self.buf.resize(payload.len(), 0);
        }
        self.buf[..payload.len()].copy_from_slice(payload);
        Ok(())
    }
    
    fn serialize_owning_to_buf(&mut self, msg: &impl OwningSerializable) -> Result<usize, IrodsError> {
        self.buf.clear();
        T::rods_owning_ser(msg, self.buf.as_mut())
    }

    // This behavior is slightly different since you
    // don't need to pass length as an argument
    pub fn pull_header(&mut self) -> Result<OwningStandardHeader, IrodsError> {
        let header_size = u32::from_be_bytes(self.read_from_server(4)?.try_into().unwrap()) as usize;
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

    pub fn push_owning<M>(&mut self, msg: &M) -> Result<(), IrodsError>
    where
        M: OwningSerializable,
    {
        let serialiazed_bytes = T::rods_owning_ser(msg, self.buf.as_mut_slice())?;
        self.socket.write_all(&self.buf[..serialiazed_bytes])?;
        Ok(())
    }
}
