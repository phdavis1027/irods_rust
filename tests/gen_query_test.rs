mod test_common;
use std::path::Path;

use deadpool::managed;
use futures::{pin_mut, StreamExt};
use irods_client::{
    bosd::xml::XML,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    fs::download::ParallelDownloadContext,
};
use test_common::test_manager;

#[tokio::test]
async fn gen_query_test() {
    let mut pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);
    let mut conn = pool.get().await.unwrap();

    conn.change_user_password(
        "rods".to_string(),
        "tempZone".to_string(),
        "bods".to_string(),
    )
    .await
    .unwrap();
}
