use crate::bosd::ProtocolEncoding;
use crate::error::errors::IrodsError;
use std::net::SocketAddr;
use tokio::net::TcpStream as AsyncTcpStream;

use super::{
    connect::Connect, Account, ResourceBundle, UnauthenticatedConnection, UninitializedConnection,
};

#[derive(Clone)]
pub struct TcpConnector {
    addr: SocketAddr,
}

impl TcpConnector {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

impl<T> Connect<T> for TcpConnector
where
    T: ProtocolEncoding + Send + Sync + 'static,
{
    type Transport = AsyncTcpStream;

    async fn connect(
        &self,
        account: Account,
    ) -> Result<UnauthenticatedConnection<T, Self::Transport>, IrodsError> {
        let tcp_resources = ResourceBundle::new(AsyncTcpStream::connect(self.addr).await?);

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
            "rust".to_string(),
        )
        .await?;

        let version = conn.get_version().await?;

        Ok(conn.into_unauthenticated(version))
    }
}
