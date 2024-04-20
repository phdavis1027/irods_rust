use deadpool::managed;
use irods_client::{
    bosd::xml::XML,
    connection::{authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector},
    exec_rule::Rule,
};

mod test_common;

#[tokio::test]
async fn exec_rule_test() {
    let pool = test_pool!(
        test_common::test_manager::<XML, TcpConnector, NativeAuthenticator>(),
        16
    );

    let mut conn = pool.get().await.unwrap();

    test_common::VeryAdvancedHelloWorldRuleBuilder::default()
        .greeting1("Hello".to_string())
        .greeting2("World".to_string())
        .instance(None)
        .addr(None)
        .rods_zone(None)
        .build()
        .unwrap()
        .execute(&mut conn)
        .await
        .unwrap();
}
