use std::{time::Duration, net::TcpStream, marker::PhantomData, io};

use crate::{msg::startup_pack::BorrowingStartupPack, bosd::{BorrowingSerializer, BorrowingDeserializer, OwningSerializer, OwningDeserializer}};

use super::{Account, ConnConfig, CsNegPolicy, Connection};

impl ConnConfig<TcpStream> {
    fn new(
        buf_size: usize,
        request_timeout: Duration,
        read_timeout: Duration,
        host: String,
        port: u16,
        a_ttl: u32,
    ) -> Self {
        Self {
            a_ttl,
            buf_size,
            request_timeout,
            read_timeout,
            addr: (host, port),
            cs_neg_policy: CsNegPolicy::Refuse,
            ssl_config: None,
            phantom_transport: PhantomData,
        }
    }
}

impl Default for ConnConfig<TcpStream> {
    fn default() -> Self {
        Self {
            buf_size: 8092,
            request_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(5),
            // FIXME: Change this
            cs_neg_policy: CsNegPolicy::Refuse,
            ssl_config: None,
            addr: ("172.27.0.3".into(), 1247),
            a_ttl: 30,
            phantom_transport: PhantomData,
        }
    }
}

impl<T, S> Connection<T, S>
where
    T: BorrowingSerializer + BorrowingDeserializer + OwningSerializer + OwningDeserializer,
    S: io::Read + io::Write,
{
    fn make_startup_pack(account: &Account) -> BorrowingStartupPack {
        BorrowingStartupPack::new(
            T::as_enum(),
            0,
            0,
            &account.proxy_user,
            &account.proxy_zone,
            &account.client_user,
            &account.client_zone,
            (4, 3, 0),
            "d",
            "packe",
        )
    }
}
