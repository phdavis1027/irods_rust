pub mod authenticate;
pub mod connect;
pub mod pool;
pub mod ssl;
pub mod tcp;

use std::io::Write;
use std::marker::PhantomData;

use base64::Engine;
use futures::future::TryFutureExt;
use native_tls::{Certificate, HandshakeError, TlsConnector};
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

pub struct ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub header_buf: Vec<u8>,
    pub msg_buf: Vec<u8>,
    pub bytes_buf: Vec<u8>,
    pub error_buf: Vec<u8>,
    pub transport: S,
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(transport: S) -> Self {
        Self {
            header_buf: Vec::new(),
            msg_buf: Vec::new(),
            bytes_buf: Vec::new(),
            error_buf: Vec::new(),
            transport,
        }
    }
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(self) async fn send_handshake_header<T>(
        &mut self,
        config: &SslConfig,
    ) -> Result<(), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let header = HandshakeHeader::new(
            config.algorithm.to_string(),
            config.key_size,
            config.salt_size,
            config.hash_rounds,
        );

        let header_len = T::encode(&header, &mut self.header_buf)?;
        self.send_header_len(header_len).await?;
        self.send_from_header_buf(header_len).await?;

        Ok(())
    }

    pub(self) async fn send_shared_secret<T>(&mut self, size: usize) -> Result<(), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let shared_secret = &mut self.bytes_buf[..size];
        rand::thread_rng().fill_bytes(shared_secret);

        let header = SharedSecretHeader { size };
        let header_len = T::encode(&header, &mut self.header_buf)?;

        self.send_header_len(header_len).await?;
        self.send_from_header_buf(header_len).await?;
        self.send_from_bytes_buf(size).await?;

        Ok(())
    }

    async fn send_from_bytes_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        self.transport.write_all(&mut self.bytes_buf[..len]).await?;
        Ok(())
    }

    async fn send_header_len(&mut self, len: usize) -> Result<(), IrodsError> {
        self.transport
            .write_all(&(len as u32).to_be_bytes())
            .await?;
        Ok(())
    }

    async fn send_from_msg_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        self.transport.write_all(&mut self.msg_buf[..len]).await?;
        Ok(())
    }

    async fn send_from_header_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        self.transport
            .write_all(&mut self.header_buf[..len])
            .await?;
        Ok(())
    }

    async fn read_to_msg_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        tokio::io::copy(
            &mut (&mut self.transport).take(len as u64),
            &mut Cursor::new(&mut self.msg_buf),
        )
        .await?;

        Ok(())
    }

    async fn read_to_header_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        tokio::io::copy(
            &mut (&mut self.transport).take(len as u64),
            &mut Cursor::new(&mut self.header_buf),
        )
        .await?;
        Ok(())
    }

    async fn read_to_bytes_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        tokio::io::copy(
            &mut (&mut self.transport).take(len as u64),
            &mut Cursor::new(&mut self.bytes_buf),
        )
        .await?;

        Ok(())
    }

    async fn read_to_error_buf(&mut self, len: usize) -> Result<(), IrodsError> {
        tokio::io::copy(
            &mut (&mut self.transport).take(len as u64),
            &mut Cursor::new(&mut self.error_buf),
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn read_into_buf(
        &mut self,
        sink: &mut Vec<u8>,
        len: usize,
    ) -> Result<(), IrodsError> {
        tokio::io::copy(
            &mut (&mut self.transport).take(len as u64),
            &mut Cursor::new(sink),
        )
        .await?;

        Ok(())
    }

    pub(crate) async fn read_standard_header<T>(&mut self) -> Result<StandardHeader, IrodsError>
    where
        T: ProtocolEncoding,
    {
        self.read_to_header_buf(4).await?;
        let header_len = u32::from_be_bytes(self.header_buf[..4].try_into().unwrap()) as usize;
        self.read_to_header_buf(header_len).await?;

        Ok(T::decode(&self.header_buf[..header_len])?)
    }

    pub(crate) async fn read_msg<T, M>(&mut self, len: usize) -> Result<M, IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        self.read_to_msg_buf(len).await?;
        Ok(T::decode(&self.msg_buf[..len])?)
    }

    pub(crate) async fn send_standard_header<T>(
        &mut self,
        header: StandardHeader,
    ) -> Result<(), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let len = T::encode(&header, &mut self.header_buf)?;

        self.send_header_len(len).await?;
        self.send_from_header_buf(len).await?;

        Ok(())
    }

    pub(crate) async fn send_msg<T, M>(&mut self, msg: M) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        let len = T::encode(&msg, &mut self.msg_buf)?;

        self.send_from_msg_buf(len).await?;

        Ok(self)
    }

    pub(crate) async fn get_header_and_msg<T, M>(
        &mut self,
    ) -> Result<(StandardHeader, M), IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        let header = self.read_standard_header::<T>().await?;
        let msg = self.read_msg::<T, M>(header.msg_len as usize).await?;

        Ok((header, msg))
    }

    pub(crate) async fn send_header_then_msg<T, M>(
        &mut self,
        msg: &M,
        msg_type: MsgType,
        int_info: i32,
    ) -> Result<(), IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        let msg_len = T::encode(msg, &mut self.msg_buf)?;

        let header = StandardHeader::new(msg_type, msg_len, 0, 0, int_info);

        self.send_standard_header::<T>(header).await?;
        self.send_from_msg_buf(msg_len).await?;

        Ok(())
    }
}

impl<T> UninitializedConnection<T, TcpStream>
where
    T: ProtocolEncoding,
{
    pub async fn into_tls(
        self,
        connector: TlsConnector,
        domain: &str,
    ) -> Result<UninitializedConnection<T, tokio_native_tls::TlsStream<TcpStream>>, IrodsError>
    {
        let tcp_stream = self.resources.transport;

        let async_connector = tokio_native_tls::TlsConnector::from(connector);
        let tls_stream = async_connector.connect(domain, tcp_stream).await?;

        Ok(UninitializedConnection {
            resources: ResourceBundle {
                header_buf: self.resources.header_buf,
                msg_buf: self.resources.msg_buf,
                bytes_buf: self.resources.bytes_buf,
                error_buf: self.resources.error_buf,
                transport: tls_stream,
            },
            account: self.account,
            phantom_protocol: PhantomData,
        })
    }

    pub(crate) async fn send_use_tcp(mut self) -> Result<Self, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_TCP),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_server_cs_neg(
        &mut self,
    ) -> Result<(StandardHeader, ServerCsNeg), IrodsError> {
        self.resources.get_header_and_msg::<T, _>().await
    }

    pub(crate) async fn send_use_ssl(&mut self) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_SSL),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(())
    }
}

impl<T> UninitializedConnection<T, tokio_native_tls::TlsStream<TcpStream>>
where
    T: ProtocolEncoding,
{
    pub(crate) async fn send_shared_secret(&mut self, size: usize) -> Result<(), IrodsError> {
        self.resources.send_shared_secret::<T>(size).await?;

        Ok(())
    }
}

pub struct UninitializedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    resources: ResourceBundle<C>,
    account: Account,
    phantom_protocol: PhantomData<T>,
}

impl<T, C> UninitializedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(crate) fn into_unauthenticated(self) -> UnauthenticatedConnection<T, C> {
        UnauthenticatedConnection::new(self.account, self.resources)
    }

    pub fn new(account: Account, resources: ResourceBundle<C>) -> Self {
        Self {
            resources,
            account,
            phantom_protocol: PhantomData,
        }
    }

    pub(crate) async fn send_startup_pack(
        &mut self,
        reconnect_flag: u32,
        connect_cnt: u32,
        proxy_user: String,
        proxy_zone: String,
        client_user: String,
        client_zone: String,
        rel_version: (u8, u8, u8),
        option: String,
    ) -> Result<(), IrodsError> {
        self.resources
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

        Ok(())
    }

    pub(crate) async fn send_negotiation_failed(&mut self) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(0, CsNegResult::CS_NEG_FAILURE),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn get_version(&mut self) -> Result<(), IrodsError> {
        let (header, version) = self.resources.get_header_and_msg::<T, Version>().await?;
        // TODO: Check version
        Ok(())
    }

    pub(crate) async fn send_handshake_header(
        &mut self,
        config: &SslConfig,
    ) -> Result<(), IrodsError> {
        self.resources.send_handshake_header::<T>(config).await?;

        Ok(())
    }

    pub(crate) async fn create_cert(
        &mut self,
        config: &SslConfig,
    ) -> Result<Certificate, IrodsError> {
        File::open(&config.cert_file)
            .await?
            .read_to_end(&mut self.resources.bytes_buf)
            .await?;

        Ok(Certificate::from_pem(&self.resources.bytes_buf)?)
    }
}

pub struct UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    resources: ResourceBundle<C>,
    account: Account,
    phantom_protocol: PhantomData<T>,
}

impl<T, C> UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(crate) async fn send_auth_request(
        &mut self,
        authenticator: &NativeAuthenticator,
    ) -> Result<(), IrodsError> {
        // BytesBuf = unencoded buf
        // ErrorBuf = encoded buf
        let mut unencoded_cursor = Cursor::new(&mut self.resources.bytes_buf);

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
            authenticator.a_ttl, self.account.client_user, self.account.client_zone
        )?;

        let unencoded_len = unencoded_cursor.position() as usize;
        self.resources
            .error_buf
            .resize(4 * (unencoded_len / 3 + 4), 0);

        let payload_len = authenticator
            .b64_engine
            .encode_slice(
                &unencoded_cursor.get_mut()[..unencoded_len],
                self.resources.error_buf.as_mut_slice(),
            ) // FIXME: This sucks
            .map_err(|e| IrodsError::Other(format!("{}", e)))?;

        let encoded_str = std::str::from_utf8(&self.resources.error_buf[..payload_len])?;

        self.resources
            .send_header_then_msg::<T, _>(
                &BinBytesBuf::new(encoded_str),
                MsgType::RodsApiReq,
                APN::Authentication as i32,
            )
            .await?;

        Ok(())
    }

    pub(crate) async fn get_auth_response(&mut self) -> Result<BinBytesBuf, IrodsError> {
        let (_, challenge) = self
            .resources
            .get_header_and_msg::<T, BinBytesBuf>()
            .await?;

        Ok(challenge)
    }

    pub fn new(account: Account, resources: ResourceBundle<C>) -> Self {
        Self {
            resources,
            account,
            phantom_protocol: PhantomData,
        }
    }

    pub(crate) fn into_connection(self, signature: Vec<u8>) -> Connection<T, C> {
        Connection::new(self.account, self.resources, signature)
    }
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(account: Account, resources: ResourceBundle<C>, signature: Vec<u8>) -> Self {
        Self {
            resources,
            account,
            signature,
            phantom_protocol: PhantomData,
        }
    }
}

pub struct Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(crate) resources: ResourceBundle<C>,
    pub(crate) account: Account,
    pub(crate) signature: Vec<u8>,
    pub(crate) phantom_protocol: PhantomData<T>,
}
