use std::{io, path::Path};

use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::ProtocolEncoding,
    common::{cond_input_kw::CondInputKw, APN},
    connection::Connection,
    msg::{data_obj_inp::DataObjInp, header::MsgType},
};

use super::{DataObjectHandle, OpenFlag, OprType};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn open_request<'conn>(&'conn mut self, path: &'conn Path) -> OpenRequest<'conn, T, C> {
        OpenRequest::new(self, path)
    }

    fn make_open_data_obj_inp(path: &Path, flags: i32, resc: Option<&str>) -> DataObjInp {
        let mut inp = DataObjInp::new(path.to_str().unwrap().to_owned(), OprType::No, flags, 0);
        if let Some(r) = resc {
            inp.cond_input
                .add_kw(CondInputKw::RescNameKw, r.to_string());
        }
        inp.data_size = -1;

        inp
    }

    async fn open_inner(
        &mut self,
        path: &Path,
        flags: i32,
        resc: Option<&str>,
    ) -> Result<DataObjectHandle, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_open_data_obj_inp(path, flags, resc),
                MsgType::RodsApiReq,
                APN::DataObjOpen as i32,
            )
            .await?;
        let header = self.resources.read_standard_header::<T>().await?;

        Ok(header.int_info)
    }
}

pub struct OpenRequest<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    conn: &'conn mut Connection<T, C>,
    flags: i32,
    resc: Option<&'conn str>,
    path: &'conn Path,
}

impl<'conn, T, C> OpenRequest<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(conn: &'conn mut Connection<T, C>, path: &'conn Path) -> Self {
        Self {
            conn,
            flags: 0,
            resc: None,
            path,
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

    pub fn set_resc(mut self, resc: &'conn str) -> Self {
        self.resc = Some(resc);
        self
    }

    pub async fn execute(self) -> Result<DataObjectHandle, IrodsError> {
        self.conn.open_inner(self.path, self.flags, self.resc).await
    }
}
