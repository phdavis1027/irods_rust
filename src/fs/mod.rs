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

use std::{
    io,
    path::{Path, PathBuf},
};

use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer},
    common::{cond_input_kw::CondInputKw, APN},
    connection::{
        read_standard_header, send_borrowing_msg_and_header, send_owning_msg_and_header, Connection,
    },
    msg::{
        data_obj_inp::BorrowingDataObjInp, header::MsgType,
        opened_data_obj_inp::OwningOpenedDataObjInp,
    },
};

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

#[cfg_attr(test, derive(Debug))]
pub enum OpenFlag {
    ReadOnly = 0,
    WriteOnly = 1,
    ReadWrite = 2,
    Create = 0o100,
    Truncate = 0o1000,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
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

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
pub enum Whence {
    SeekSet = 0,
    SeekCur = 1,
    SeekEnd = 2,
}

pub type DataObjectHandle = i32;

fn create_open_inp<'s, 'r>(
    path: &'s str,
    resc: Option<&'s str>,
    open_flags: i32,
) -> BorrowingDataObjInp<'r>
where
    's: 'r,
{
    let mut req = BorrowingDataObjInp::new(path, OprType::No, open_flags, 0);

    if let Some(resc) = resc {
        req.cond_input.add_kw(CondInputKw::DestRescNameKw, resc);
    };

    req.data_size = -1;

    req
}

impl<T, C> Connection<T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    pub fn close<'s>(&mut self, fd: DataObjectHandle) -> Result<(), IrodsError> {
        send_owning_msg_and_header::<T, _, _>(
            &mut self.connector,
            OwningOpenedDataObjInp::new(fd, 0, Whence::SeekSet, OprType::No, 0, 0),
            MsgType::RodsApiReq,
            APN::DataObjClose as i32,
            &mut self.msg_buf,
            &mut self.header_buf,
        )?;

        Ok(())
    }

    pub fn open_request<'s, 'conn>(
        &'conn mut self,
        path: &'s Path,
    ) -> OpenRequest<'s, 'conn, T, C> {
        OpenRequest::new(self, path)
    }

    fn open_inner<'s>(
        &mut self,
        path: &'s str,
        flags: i32,
        resc: Option<&'s str>,
    ) -> Result<DataObjectHandle, IrodsError> {
        let req = create_open_inp(path, resc, flags);

        send_borrowing_msg_and_header::<T, _, _>(
            &mut self.connector,
            req,
            MsgType::RodsApiReq,
            APN::DataObjOpen as i32,
            &mut self.header_buf,
            &mut self.msg_buf,
        )?;

        Ok(read_standard_header::<_, T>(&mut self.header_buf, &mut self.connector)?.int_info)
    }
}

pub struct OpenRequest<'s, 'conn, T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    conn: &'conn mut Connection<T, C>,
    path: &'s Path,
    resc: Option<&'s str>,
    flags: i32,
}

impl<'s, 'conn, T, C> OpenRequest<'s, 'conn, T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    pub fn new(conn: &'conn mut Connection<T, C>, path: &'s Path) -> Self {
        Self {
            conn,
            path,
            flags: 0,
            resc: None,
        }
    }

    pub fn set_flag(mut self, flag: OpenFlag) -> Self {
        self.flags |= flag as i32;
        self
    }

    pub fn unset_flag(mut self, flag: OpenFlag) -> Self {
        self.flags &= !(flag as i32);
        self
    }

    pub fn resc(mut self, resc: &'s str) -> Self {
        self.resc = Some(resc);
        self
    }

    pub fn execute(self) -> Result<DataObjectHandle, IrodsError> {
        //FIXME: Validate UTF-8
        self.conn
            .open_inner(self.path.to_str().unwrap(), self.flags, self.resc)
    }
}
