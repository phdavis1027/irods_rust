use std::{
    borrow::BorrowMut,
    io::{BufReader, Read, Write},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

use futures::TryFutureExt;
use native_tls::TlsConnector;
use crate::error::errors::IrodsError;
use tokio::net::TcpStream as AsyncTcpStream;
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
use tokio_native_tls::{TlsConnector as AsyncTlsConnector, TlsStream as AsyncTlsStream};

use crate::{
    bosd::{xml::XML, ProtocolEncoding},
    common::{CsNegPolicy, CsNegResult},
};

use super::{connect::Connect, ResourceBundle, UnauthenticatedConnection, UninitializedConnection};

pub struct SslConnector {
    inner: Arc<SslConnectorInner>,
}

pub struct SslConfig {
    pub cert_file: PathBuf,
    pub domain: String,
    pub key_size: usize,
    pub salt_size: usize,
    pub hash_rounds: usize,
    pub algorithm: String,
}

impl SslConfig {
    pub fn new(
        cert_file: PathBuf,
        domain: String,
        key_size: usize,
        salt_size: usize,
        hash_rounds: usize,
        algorithm: String,
    ) -> Self {
        Self {
            cert_file,
            domain,
            key_size,
            salt_size,
            hash_rounds,
            algorithm,
        }
    }

    #[cfg(test)]
    pub fn test_config() -> Self {
        Self {
            cert_file: PathBuf::from("server.crt"),
            domain: "172.18.0.3".to_string(),
            key_size: 32,
            salt_size: 16,
            hash_rounds: 8,
            algorithm: "AES-256-CBC".to_string(),
        }
    }
}

pub struct SslConnectorInner {
    pub config: SslConfig,
    pub addr: SocketAddr,
}

impl Clone for SslConnector {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl SslConnector {
    pub fn new(addr: SocketAddr, config: SslConfig) -> Self {
        Self {
            inner: Arc::new(SslConnectorInner { config, addr }),
        }
    }
}

pub trait IntoConnection: Send {}

impl<T> Connect<T> for SslConnector
where
    T: ProtocolEncoding + Send,
{
    type Transport = AsyncTlsStream<AsyncTcpStream>;

    async fn connect(
        &self,
        account: super::Account,
    ) -> Result<UnauthenticatedConnection<T, Self::Transport>, IrodsError> {
        let tcp_resources = ResourceBundle::new(AsyncTcpStream::connect(self.inner.addr).await?);

        let mut conn: UninitializedConnection<T, AsyncTcpStream> =
            UninitializedConnection::new(account.clone(), tcp_resources);

        conn.send_startup_pack(
            0,
            0,
            account.proxy_user.clone(),
            account.proxy_zone.clone(),
            account.client_user.clone(),
            account.client_zone.clone(),
            (4, 3, 2),
            "rust;request_server_negotiation".to_string(),
        )
        .await?;
        conn.get_server_cs_neg().await?;
        conn.send_use_ssl().await?;
        conn.get_version().await?;

        let cert = conn.create_cert(&self.inner.config).await?;

        let blocking_connector = TlsConnector::builder()
            .add_root_certificate(cert)
            .danger_accept_invalid_certs(true) // FIXME: Only for testing
            .build()
            .unwrap();

        let mut conn = conn
            .into_tls(blocking_connector, &self.inner.config.domain)
            .await?;
        conn.send_handshake_header(&self.inner.config).await?;
        conn.send_shared_secret(self.inner.config.key_size).await?;

        Ok(conn.into_unauthenticated())
    }
}
