use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use irods_client::{
    bosd::{xml::XML, ProtocolEncoding},
    connection::{
        authenticate::{Authenticate, NativeAuthenticator},
        connect::Connect,
        pool::IrodsManager,
        tcp::TcpConnector,
        Account,
    },
};

pub fn test_manager<T, C, A>() -> IrodsManager<XML, TcpConnector, NativeAuthenticator>
where
    T: ProtocolEncoding + Send + Sync,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    let account = Account {
        client_user: "rods".to_string(),
        client_zone: "tempZone".to_string(),
        proxy_user: "rods".to_string(),
        proxy_zone: "tempZone".to_string(),
        password: "rods".to_string(),
    };

    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 18, 0, 3), 1247));

    let connector = TcpConnector::new(addr);

    let authenticator = NativeAuthenticator::new(30, "rods".to_string());

    IrodsManager::new(account, connector, authenticator, 10, 10)
}

#[macro_export]
macro_rules! test_pool {
    ($manager:expr, $size:expr) => {{
        let pool: managed::Pool<IrodsManager<_, _, _>> = managed::Pool::builder($manager)
            .max_size($size)
            .build()
            .unwrap();
        pool
    }};
}
