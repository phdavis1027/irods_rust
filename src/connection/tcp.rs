use crate::bosd::ProtocolEncoding;
use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;
use std::net::SocketAddr;
use tokio::net::TcpStream as AsyncTcpStream;

use super::{connect::Connect, Account, Connection, ResourceBundle, UnauthenticatedConnection};

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
    ) -> Result<super::UnauthenticatedConnection<T, Self::Transport>, IrodsError> {
        let tcp_resources = ResourceBundle::new(AsyncTcpStream::connect(self.addr).await?);

        let conn: UnauthenticatedConnection<T, AsyncTcpStream> =
            UnauthenticatedConnection::new(account.clone(), tcp_resources);

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
        .and_then(|conn| conn.get_version())
        .await
    }
}
