use std::time::{Duration, Instant};

use async_trait::async_trait;
use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use deadpool_runtime::Runtime;
use deadpool_sync::SyncWrapper;
use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::{
    BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer,
};

use super::{authenticate::Authenticate, connect::Connect, Account, Connection};

pub struct IrodsManager<T, C, A>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    T: Send,
    C: Connect<T>,
    A: Authenticate<T, C::Transport>,
{
    account: Account,
    authenticator: A,
    connector: C,
    phantom: std::marker::PhantomData<T>,
    num_secs_before_refresh: Duration,
    num_recycles_before_refresh: usize,
}

impl<T, C, A> IrodsManager<T, C, A>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    T: Send,
    C: Connect<T>,
    A: Authenticate<T, C::Transport>,
{
    pub fn new(
        account: Account,
        authenticator: A,
        connector: C,
        num_secs_before_refresh: usize,
        num_recycles_before_refresh: usize,
    ) -> Self {
        Self {
            account,
            authenticator,
            connector,
            num_secs_before_refresh: Duration::from_secs(num_secs_before_refresh as u64),
            num_recycles_before_refresh,
            phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<T, C, A> Manager for IrodsManager<T, C, A>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    T: Send + Sync + 'static,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    type Type = Connection<T, C::Transport>;
    type Error = IrodsError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {}

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
