pub mod authenticate;
pub mod connect;
pub mod pool;
pub mod ssl;
pub mod tcp;

use std::io::Write;
use std::marker::PhantomData;

use base64::Engine;
use futures::future::TryFutureExt;
use native_tls::{Certificate, TlsConnector};
use rand::RngCore;
use rods_prot_msg::error::errors::IrodsError;
use std::io::Cursor;
use tokio::fs::File;
use tokio_native_tls::TlsStream;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::common::APN;
use crate::msg::bin_bytes_buf::BinBytesBuf;
use crate::msg::header::{HandshakeHeader, SharedSecretHeader};
use crate::msg::version::Version;
use crate::{
    bosd::{Deserializable, ProtocolEncoding, Serialiazable},
    common::CsNegResult,
    msg::{
        cs_neg::{ClientCsNeg, ServerCsNeg},
        header::{MsgType, StandardHeader},
        startup_pack::StartupPack,
    },
};

use self::authenticate::NativeAuthenticator;
use self::ssl::SslConfig;

const MAX_PASSWORD_LEN: usize = 50;

#[derive(Clone)]
pub struct Account {
    pub client_user: String,
    pub client_zone: String,
    pub proxy_user: String,
    pub proxy_zone: String,
}

impl Account {
    #[cfg(test)]
    pub fn test_account() -> Self {
        Self {
            client_user: "rods".to_string(),
            client_zone: "tempZone".to_string(),
            proxy_user: "rods".to_string(),
            proxy_zone: "tempZone".to_string(),
        }
    }
}

pub struct UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    inner: Box<UnauthenticatedConnectionInner<T, C>>,
}

pub struct UnauthenticatedConnectionInner<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    resources: ResourceBundle<C>,
    account: Account,
    phantom_protocol: PhantomData<T>,
}

pub struct ResourceBundleInner<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub header_buf: Vec<u8>,
    pub msg_buf: Vec<u8>,
    pub bytes_buf: Vec<u8>,
    pub error_buf: Vec<u8>,
    pub connector: S,
}

#[repr(transparent)]
pub struct ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    inner: Box<ResourceBundleInner<S>>,
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(connector: S) -> Self {
        Self {
            inner: Box::new(ResourceBundleInner {
                header_buf: Vec::new(),
                msg_buf: Vec::new(),
                bytes_buf: Vec::new(),
                error_buf: Vec::new(),
                connector,
            }),
        }
    }
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(self) async fn send_shared_secret<T>(&mut self, size: usize) -> Result<(), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let mut shared_secret = &mut self.inner.msg_buf[..size];
        rand::thread_rng().fill_bytes(shared_secret);

        self.send_msg::<T, SharedSecretHeader>(SharedSecretHeader { size })
            .and_then(|mut this| Self::send_from_msg_buf(&mut this.inner, size))
            .await?;

        Ok(())
    }

    fn map_into_transport<D>(self, transport: D) -> ResourceBundle<D>
    where
        D: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        ResourceBundle {
            inner: Box::new(ResourceBundleInner {
                header_buf: self.inner.header_buf,
                msg_buf: self.inner.msg_buf,
                bytes_buf: self.inner.bytes_buf,
                error_buf: self.inner.error_buf,
                connector: transport,
            }),
        }
    }

    async fn send_header_len(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner
            .connector
            .write_all(&(len as u32).to_be_bytes())
            .await?;
        Ok(inner)
    }

    async fn send_from_msg_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner.connector.write_all(&mut inner.msg_buf[..len]).await?;
        Ok(inner)
    }

    async fn send_from_header_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner
            .connector
            .write_all(&mut inner.header_buf[..len])
            .await?;
        Ok(inner)
    }

    async fn read_to_msg_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.msg_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_header_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.header_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_bytes_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.bytes_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_error_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.error_buf),
        )
        .await?;
        Ok(inner)
    }

    pub(crate) async fn read_standard_header<T>(
        &mut self,
    ) -> Result<(StandardHeader, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let header = Self::read_to_header_buf(&mut self.inner, 4)
            .and_then(|inner| async {
                let header_len =
                    u32::from_be_bytes(inner.header_buf[..4].try_into().unwrap()) as usize;

                let inner = Self::read_to_header_buf(inner, header_len).await?;

                Ok(T::decode(&inner.header_buf[..header_len])?)
            })
            .await?;

        println!("Received header: {:?}", header);

        Ok((header, self))
    }

    pub(crate) async fn read_msg<T, M>(&mut self, len: usize) -> Result<(M, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        let msg = async {
            let inner = Self::read_to_msg_buf(&mut self.inner, len).await?;
            Ok::<_, IrodsError>(T::decode(&inner.msg_buf[..len])?)
        }
        .await?;

        println!("Received message: {:?}", msg);

        Ok((msg, self))
    }

    pub(crate) async fn send_standard_header<T>(
        &mut self,
        header: StandardHeader,
    ) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
    {
        println!("Sending header: {:?}", header);
        let len = T::encode(&header, &mut self.inner.header_buf)?;

        Self::send_header_len(&mut self.inner, len)
            .and_then(|inner| Self::send_from_header_buf(inner, len))
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_msg<T, M>(&mut self, msg: M) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        println!("Sending message: {:?}", msg);

        let len = T::encode(&msg, &mut self.inner.msg_buf)?;

        Self::send_from_msg_buf(&mut self.inner, len).await?;

        Ok(self)
    }

    pub(crate) async fn get_header_and_msg<T, M>(
        &mut self,
    ) -> Result<(StandardHeader, M, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        let (header, _) = self.read_standard_header::<T>().await?;

        let (msg, this) = self.read_msg::<T, M>(header.msg_len as usize).await?;

        Self::read_to_bytes_buf(&mut this.inner, header.bs_len as usize)
            .and_then(|inner| Self::read_to_error_buf(inner, header.error_len as usize))
            .await?;

        Ok((header, msg, self))
    }

    pub(crate) async fn send_header_then_msg<T, M>(
        &mut self,
        msg: &M,
        msg_type: MsgType,
        int_info: i32,
    ) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        let msg_len = T::encode(msg, &mut self.inner.msg_buf)?;

        let header = StandardHeader::new(msg_type, msg_len, 0, 0, int_info);

        Self::send_standard_header::<T>(self, header)
            .and_then(|this| Self::send_from_msg_buf(&mut this.inner, msg_len))
            .await?;

        Ok(self)
    }
}

impl<T> UnauthenticatedConnection<T, TcpStream>
where
    T: ProtocolEncoding,
{
    pub async fn into_tls(
        self,
        connector: TlsConnector,
        domain: &str,
    ) -> Result<UnauthenticatedConnection<T, tokio_native_tls::TlsStream<TcpStream>>, IrodsError>
    {
        let tcp_stream = self.inner.resources.inner.connector;

        let async_connector = tokio_native_tls::TlsConnector::from(connector);
        let tls_stream = async_connector.connect(domain, tcp_stream).await?;

        Ok(UnauthenticatedConnection {
            inner: Box::new(UnauthenticatedConnectionInner {
                resources: ResourceBundle {
                    inner: Box::new(ResourceBundleInner {
                        header_buf: self.inner.resources.inner.header_buf,
                        msg_buf: self.inner.resources.inner.msg_buf,
                        bytes_buf: self.inner.resources.inner.bytes_buf,
                        error_buf: self.inner.resources.inner.error_buf,
                        connector: tls_stream,
                    }),
                },
                account: self.inner.account,
                phantom_protocol: PhantomData,
            }),
        })
    }
}

impl<T, C> UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(account: Account, resources: ResourceBundle<C>) -> Self {
        Self {
            inner: Box::new(UnauthenticatedConnectionInner {
                resources,
                account,
                phantom_protocol: PhantomData,
            }),
        }
    }

    pub(crate) fn into_authenticated(self, signature: Vec<u8>) -> Connection<T, C> {
        Connection::new(self.inner.account, self.inner.resources, signature)
    }

    pub(crate) async fn send_startup_pack(
        mut self,
        reconnect_flag: u32,
        connect_cnt: u32,
        proxy_user: String,
        proxy_zone: String,
        client_user: String,
        client_zone: String,
        rel_version: (u8, u8, u8),
        option: String,
    ) -> Result<Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &StartupPack::new(
                    T::as_enum(),
                    reconnect_flag,
                    connect_cnt,
                    proxy_user,
                    proxy_zone,
                    client_user,
                    client_zone,
                    rel_version,
                    option,
                ),
                MsgType::RodsConnect,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_server_cs_neg(
        mut self,
    ) -> Result<(StandardHeader, ServerCsNeg, Self), IrodsError> {
        let (header, msg, _) = self.inner.resources.get_header_and_msg::<T, _>().await?;

        Ok((header, msg, self))
    }

    pub(crate) async fn send_use_ssl(mut self) -> Result<Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_SSL),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_use_tcp(mut self) -> Result<Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_TCP),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_negotiation_failed(mut self) -> Result<Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(0, CsNegResult::CS_NEG_FAILURE),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_version(mut self) -> Result<Self, IrodsError> {
        let (header, version, _) = self
            .inner
            .resources
            .get_header_and_msg::<T, Version>()
            .await?;

        // TODO: Check header and version

        Ok(self)
    }

    pub(crate) async fn create_cert(
        &mut self,
        config: &SslConfig,
    ) -> Result<Certificate, IrodsError> {
        File::open(&config.cert_file)
            .await?
            .read_to_end(&mut self.inner.resources.inner.bytes_buf)
            .await?;

        Ok(Certificate::from_pem(
            &self.inner.resources.inner.bytes_buf,
        )?)
    }

    pub(crate) async fn send_auth_request(
        mut self,
        authenticator: &NativeAuthenticator,
    ) -> Result<Self, IrodsError> {
        // BytesBuf = unencoded buf
        // ErrorBuf = encoded buf
        let mut unencoded_cursor = Cursor::new(&mut self.inner.resources.inner.bytes_buf);

        write!(
            unencoded_cursor,
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
            authenticator.a_ttl, self.inner.account.client_user, self.inner.account.client_zone
        )?;

        let unencoded_len = unencoded_cursor.position() as usize;
        self.inner
            .resources
            .inner
            .error_buf
            .resize(4 * (unencoded_len / 3 + 4), 0);

        let payload_len = authenticator
            .b64_engine
            .encode_slice(
                &unencoded_cursor.get_mut()[..unencoded_len],
                self.inner.resources.inner.error_buf.as_mut_slice(),
            ) // FIXME: This sucks
            .map_err(|e| IrodsError::Other(format!("{}", e)))?;

        let encoded_str =
            std::str::from_utf8(&self.inner.resources.inner.error_buf[..payload_len])?;

        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &BinBytesBuf::new(encoded_str),
                MsgType::RodsApiReq,
                APN::Authentication as i32,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_auth_response(mut self) -> Result<(BinBytesBuf, Self), IrodsError> {
        let (header, challenge, _) = self
            .inner
            .resources
            .get_header_and_msg::<T, BinBytesBuf>()
            .await?;

        Ok((challenge, self))
    }
}

impl<T> UnauthenticatedConnection<T, TlsStream<TcpStream>>
where
    T: ProtocolEncoding,
{
    pub(crate) async fn send_handshake_header(
        mut self,
        config: &SslConfig,
    ) -> Result<Self, IrodsError> {
        self.inner
            .resources
            .send_msg::<T, _>(HandshakeHeader::new(
                config.algorithm.clone(),
                config.key_size,
                config.salt_size,
                config.hash_rounds,
            ))
            .await?;
        Ok(self)
    }

    pub(crate) async fn send_shared_secret(mut self, size: usize) -> Result<Self, IrodsError> {
        self.inner.resources.send_shared_secret::<T>(size).await?;

        Ok(self)
    }
}

pub struct Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    inner: Box<ConnectionInner<T, C>>,
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(account: Account, resources: ResourceBundle<C>, signature: Vec<u8>) -> Self {
        Self {
            inner: Box::new(ConnectionInner {
                resources,
                account,
                signature,
                phantom_protocol: PhantomData,
            }),
        }
    }
}

pub struct ConnectionInner<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    resources: ResourceBundle<C>,
    account: Account,
    signature: Vec<u8>,
    phantom_protocol: PhantomData<T>,
}
