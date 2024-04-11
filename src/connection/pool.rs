use std::time::Duration;

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleError, RecycleResult};
use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::ProtocolEncoding;

use super::{authenticate::Authenticate, connect::Connect, Account, Connection};

pub struct IrodsManager<T, C, A>
where
    T: ProtocolEncoding + Send,
    C: Connect<T>,
    A: Authenticate<T, C::Transport>,
{
    account: Account,
    connector: C,
    authenticator: A,
    phantom: std::marker::PhantomData<T>,
    num_secs_before_refresh: Duration,
    num_recycles_before_refresh: usize,
}

impl<T, C, A> IrodsManager<T, C, A>
where
    T: ProtocolEncoding + Send,
    C: Connect<T>,
    A: Authenticate<T, C::Transport>,
{
    pub fn new(
        account: Account,
        connector: C,
        authenticator: A,
        num_secs_before_refresh: usize,
        num_recycles_before_refresh: usize,
    ) -> Self {
        Self {
            account,
            connector,
            authenticator,
            num_secs_before_refresh: Duration::from_secs(num_secs_before_refresh as u64),
            num_recycles_before_refresh,
            phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, C, A> Manager for IrodsManager<T, C, A>
where
    T: ProtocolEncoding + Send + Sync,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    type Type = Connection<T, C::Transport>;
    type Error = IrodsError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        self.connector
            .connect(self.account.clone())
            .and_then(|unauth_conn| self.authenticator.authenticate(unauth_conn))
            .await
    }

    async fn recycle(
        &self,
        conn: &mut Self::Type,
        metrics: &deadpool::managed::Metrics,
    ) -> RecycleResult<Self::Error> {
        if metrics.recycle_count >= self.num_recycles_before_refresh {
            return Err(RecycleError::StaticMessage("Recycles limit reached"));
        }

        if metrics.created.elapsed() >= self.num_secs_before_refresh {
            return Err(RecycleError::StaticMessage("Time limit reached"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use deadpool::managed;

    use crate::{
        bosd::xml::XML,
        connection::{
            authenticate::NativeAuthenticator,
            ssl::{SslConfig, SslConnector},
            tcp::TcpConnector,
        },
    };

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_ssl() {
        let account = Account::test_account();
        let ssl_config = SslConfig::test_config();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 18, 0, 3), 1247));

        let connector = SslConnector::new(addr, ssl_config);
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let manager: IrodsManager<XML, SslConnector, NativeAuthenticator> =
            IrodsManager::new(account, connector, authenticator, 10, 10);

        let pool: managed::Pool<IrodsManager<_, _, _>> = managed::Pool::builder(manager)
            .max_size(16)
            .build()
            .unwrap();

        let _ = pool.get().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp() {
        let account = Account::test_account();

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 18, 0, 3), 1247));
        let connector = TcpConnector::new(addr);
        let authenticator = NativeAuthenticator::new(30, "rods".into());
        let manager: IrodsManager<XML, TcpConnector, NativeAuthenticator> =
            IrodsManager::new(account, connector, authenticator, 10, 10);

        let pool: managed::Pool<IrodsManager<_, _, _>> = managed::Pool::builder(manager)
            .max_size(16)
            .build()
            .unwrap();

        let _ = pool.get().await.unwrap();
    }
}
