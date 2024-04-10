use std::{
    fs::File,
    io::{BufReader, Read, Write},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

use futures::TryFutureExt;
use rand::{random, RngCore};
use rods_prot_msg::error::errors::IrodsError;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_native_tls::{TlsConnector, TlsStream};

use crate::{
    bosd::{xml::XML, ProtocolEncoding},
    common::{CsNegPolicy, CsNegResult},
};

use super::{connect::Connect, ResourceBundle, UnauthenticatedConnection};

pub struct SslConnector {
    inner: Arc<SslConnectorInner>,
}

pub struct SslConfig {
    pub cert_file: PathBuf,
    pub domain: String,
    pub key_size: usize,
    pub salt_size: usize,
    pub hash_rounds: usize,
    algorithm: String,
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

impl<T> Connect<T> for SslConnector
where
    T: ProtocolEncoding,
{
    type Transport = TlsStream<TcpStream>;

    async fn connect(
        &self,
        account: super::Account,
    ) -> Result<UnauthenticatedConnection<T, Self::Transport>, IrodsError> {
        let tcp_resources = ResourceBundle::new(TcpStream::connect(self.inner.addr).await?);

        let mut conn: UnauthenticatedConnection<T, TcpStream> =
            UnauthenticatedConnection::new(account.clone(), tcp_resources);

        let conn = conn
            .send_startup_pack(
                0,
                0,
                account.proxy_user.clone(),
                account.proxy_zone.clone(),
                account.client_user.clone(),
                account.client_zone.clone(),
                (4, 3, 2),
                "rust;request_server_negotiation;".to_string(),
            )
            .and_then(|conn| conn.get_server_cs_neg())
            .and_then(|(header, cs_neg, conn)| {
                // TODO: Check the cs_neg
                conn.send_use_ssl()
            })
            .and_then(|conn| conn.get_version())
            .await?;

        todo!()
    }
}
