mod test_common;
use std::path::Path;

use deadpool::managed;
use futures::{pin_mut, stream::StreamExt};
use irods_client::{
    bosd::xml::XML,
    common::icat_column::IcatColumn,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    fs::download::ParallelDownloadContext,
    msg::gen_query::{GenQueryInp, IcatPredicate, QueryBuilder},
};
use test_common::test_manager;

#[tokio::test]
async fn gen_query_test() {
    let mut pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);

    let home = "/tempZone/home/rods";

    let mut ctx = ParallelDownloadContext::new(
        &mut pool,
        10,
        &Path::new("/tempZone/home/rods/test_coll"),
        &Path::new("."),
    );

    ctx.recursive().force_overwrite();
    ctx.download().await.unwrap();
}
