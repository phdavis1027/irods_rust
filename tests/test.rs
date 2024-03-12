mod test_common;
use std::path::Path;

use deadpool::managed;
use irods_client::{
    bosd::xml::XML,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    fs::{download::ParallelDownloadContext, upload::ParallelTransferContext},
};
use test_common::test_manager;

#[tokio::test]
async fn gen_query_test() {
    let mut pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);

    let remote_path = "/tempZone/home/rods/totc.txt";
    let local_path = "./totc.txt";

    ParallelDownloadContext::new(&mut pool, 10, Path::new(remote_path), Path::new(local_path))
        .max_size_before_parallel(1024)
        .download()
        .await
        .unwrap();
}
