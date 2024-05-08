use std::path::Path;

use tokio::io::{AsyncRead, AsyncWriteExt};

use crate::{
    bosd::ProtocolEncoding,
    common::{self, cond_input_kw::CondInputKw, ObjectType, APN},
    connection::Connection,
    error::errors::IrodsError,
    msg::{coll::CollInp, data_obj_inp::DataObjInp, header::MsgType},
};

use super::OprType;

pub struct DeleteRequest<'conn, 'p, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    conn: &'conn mut Connection<T, C>,
    path: &'p Path,
    force: bool,
    recursive: bool,
}

impl<'conn, 'p, T, C> DeleteRequest<'conn, 'p, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(conn: &'conn mut Connection<T, C>, path: &'p Path) -> Self {
        Self {
            conn,
            path,
            force: false,
            recursive: false,
        }
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub async fn execute(self) -> Result<(), IrodsError> {
        self.conn
            .delete(self.path, self.force, self.recursive)
            .await
    }
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn make_delete_data_object_request(path: &Path, force: bool) -> DataObjInp {
        let mut inp = DataObjInp::new(path.to_str().unwrap().to_owned(), OprType::No, 0, 0);

        if force {
            inp.cond_input.set_kw(CondInputKw::ForceFlagKw);
        }

        inp
    }

    fn make_delete_coll_request(path: &Path, force: bool, recursive: bool) -> CollInp {
        let mut inp = CollInp::builder().build();

        inp.name = path.to_str().unwrap().to_owned();

        if force {
            inp.cond_input.set_kw(CondInputKw::ForceFlagKw);
        }

        if recursive {
            inp.cond_input.set_kw(CondInputKw::RecursiveOprKw);
        }

        inp
    }

    async fn delete<'path>(
        &mut self,
        path: &'path Path,
        force: bool,
        recursive: bool,
    ) -> Result<(), IrodsError> {
        let stat = self.stat(path).await?;

        match stat.object_type {
            ObjectType::Coll => {
                self.delete_coll(path, force, recursive).await?;
            }
            ObjectType::DataObj => {
                self.delete_data_obj(path, force).await?;
            }
            _ => {
                return Err(IrodsError::Other("No such path".to_string()));
            }
        }

        Ok(())
    }

    async fn delete_coll<'path>(
        &mut self,
        path: &'path Path,
        force: bool,
        recursive: bool,
    ) -> Result<(), IrodsError> {
        let inp = Self::make_delete_coll_request(path, force, recursive);

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::RmColl as i32)
            .await?;

        while self.resources.read_standard_header::<T>().await?.int_info
            == common::response::SVR_TO_CLI_COLL_STAT
        {
            self.resources
                .transport
                .write_i32(common::response::SVR_TO_CLI_COLL_STAT_REPLY)
                .await?;
        }

        Ok(())
    }

    async fn delete_data_obj<'path>(
        &mut self,
        path: &'path Path,
        force: bool,
    ) -> Result<(), IrodsError> {
        let inp = Self::make_delete_data_object_request(path, force);

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::DataObjUnlink as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }
}
