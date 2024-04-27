mod test_common;
use std::path::Path;

use deadpool::managed;
use irods_client::{
    bosd::xml::XML,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    fs::download::ParallelDownloadContext,
};
use test_common::test_manager;

#[tokio::test]
async fn gen_query_test() {
    let mut pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);

    let mut ctx = ParallelDownloadContext::new(
        &mut pool,
        15,
        &Path::new("/tempZone/home/rods/test_coll"),
        &Path::new("/home/phillipdavis/irods_test/test_dir"),
    );

    ctx.recursive().force_overwrite();
    ctx.download().await.unwrap();
}
