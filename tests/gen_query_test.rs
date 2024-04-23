mod test_common;
use deadpool::managed;
use futures::{pin_mut, stream::StreamExt};
use irods_client::{
    bosd::xml::XML,
    common::icat_column::IcatColumn,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    gen_query::Query,
    msg::gen_query::{GenQueryInp, IcatPredicate, QueryBuilder},
};
use test_common::test_manager;

#[tokio::test]
async fn gen_query_test() {
    let pool = test_pool!(test_manager::<XML, TcpConnector, NativeAuthenticator>(), 17);

    let home = "/tempZone/home/rods";

    let inp = QueryBuilder::new()
        .select(IcatColumn::DataObjectId)
        .select(IcatColumn::DataObjectBaseName)
        .select(IcatColumn::DataObjectSize)
        .select(IcatColumn::DataObjectTypeName)
        .select(IcatColumn::DataObjectReplNum)
        .select(IcatColumn::DataObjectOwnerName)
        .select(IcatColumn::DataObjectChecksum)
        .select(IcatColumn::DataObjectReplicastatus)
        .select(IcatColumn::DataObjectRescourceName)
        .select(IcatColumn::DataObjectPhysicalPath)
        .select(IcatColumn::DataObjectResourceHierarchy)
        .select(IcatColumn::DataObjectCreateTime)
        .select(IcatColumn::DataObjectModifyTime)
        .condition(
            IcatColumn::CollectionName,
            IcatPredicate::Equals(home.to_string()),
        )
        .build();

    let mut conn = pool.get().await.unwrap();

    let query = Query::new(&mut conn, inp);
    pin_mut!(query);

    while let Some(result) = query.next().await {
        println!("{:?}", result);
    }

    while let Some(result) = query.next().await {
        println!("{:?}", result);
    }
}
