use deadpool::managed;
use irods_client::{
    bosd::xml::XML,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    exec_rule::Rule,
};

mod test_common;
use test_common::test_manager;

#[tokio::test]
async fn test_gen_query() {
    let pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);
}
