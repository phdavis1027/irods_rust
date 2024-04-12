pub mod close;
pub mod open;
pub mod read;
pub mod seek;

/*
#define S_IRWXU 0000700    /* RWX mask for owner */
#define S_IRUSR 0000400    /* R for owner */
#define S_IWUSR 0000200    /* W for owner */
#define S_IXUSR 0000100    /* X for owner */

#define S_IRWXG 0000070    /* RWX mask for group */
#define S_IRGRP 0000040    /* R for group */
#define S_IWGRP 0000020    /* W for group */
#define S_IXGRP 0000010    /* X for group */

#define S_IRWXO 0000007    /* RWX mask for other */
#define S_IROTH 0000004    /* R for other */
#define S_IWOTH 0000002    /* W for other */
#define S_IXOTH 0000001    /* X for other */

#define S_ISUID 0004000    /* set user id on execution */
#define S_ISGID 0002000    /* set group id on execution */
#define S_ISVTX 0001000    /* save swapped text even after use */
*/

#[cfg_attr(test, derive(Debug))]
pub enum CreateMode {
    OwnerRead = 0o400,
    OwnerWrite = 0o200,
    OwnerExecute = 0o100,
    GroupRead = 0o040,
    GroupWrite = 0o020,
    GroupExecute = 0o010,
    OtherRead = 0o004,
    OtherWrite = 0o002,
    OtherExecute = 0o001,
}

#[derive(Debug, Clone, Copy)]
pub enum OpenFlag {
    ReadOnly = 0,
    WriteOnly = 1,
    ReadWrite = 2,
    Create = 0o100,
    Truncate = 0o1000,
}

#[derive(Debug, Clone, Copy)]
pub enum OprType {
    No = 0,
    Done = 9999,
    Put = 1,
    Get = 2,
    SameHostCopy = 3,
    CopyToLocal = 4,
    CopyToRemote = 5,
    Replicate = 6,
    ReplicateDst = 7,
    ReplicateSrc = 8,
    CopyDst = 9,
    CopySrc = 10,
    RenameDataObj = 11,
    RenameColl = 12,
    Move = 13,
    Rsync = 14,
    PhyMove = 15,
    PhyMoveSrc = 16,
    PhyMoveDst = 17,
    QueryDataObj = 18,
    QueryDataObjRecursive = 19,
    QueryColl = 20,
    QueryCollRecursive = 21,
    RenameUnknownType = 22,
    RemoteZone = 24,
    Unreg = 26,
}

#[derive(Debug, Clone, Copy)]
pub enum Whence {
    SeekSet = 0,
    SeekCur = 1,
    SeekEnd = 2,
}

pub type DataObjectHandle = i32;

#[cfg(test)]
mod test {
    use std::{
        net::{Ipv4Addr, SocketAddr, SocketAddrV4},
        path::Path,
    };

    use deadpool::managed;
    use futures::TryFutureExt;

    use crate::{
        bosd::xml::XML,
        connection::{
            authenticate::NativeAuthenticator,
            pool::IrodsManager,
            ssl::{SslConfig, SslConnector},
            tcp::TcpConnector,
            Account,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_open_close_sync() {
        let account = Account::test_account();
        let ssl_config = SslConfig::test_config();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 18, 0, 3), 1247));

        let connector = SslConnector::new(addr, ssl_config);
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let manager: IrodsManager<XML, SslConnector, NativeAuthenticator> =
            IrodsManager::new(account, connector, authenticator, 10, 10);

        let pool: managed::Pool<IrodsManager<_, _, _>> = managed::Pool::builder(manager)
            .max_size(16)
            .build()
            .unwrap();

        let mut conn = pool.get().await.unwrap();

        println!("Successfuly got connection.");

        let path = "/tempZone/home/rods/test.txt";
        let (handle, _) = conn
            .open_request(&Path::new(path))
            .set_flag(OpenFlag::ReadOnly)
            .execute()
            .await
            .unwrap();

        conn.seek(handle, Whence::SeekSet, 10)
            .and_then(|conn| conn.close(handle))
            .await
            .unwrap();
    }
}
