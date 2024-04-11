use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    msg::{file_lseek_out::FileLseekOut, header::MsgType, opened_data_obj_inp::OpenedDataObjInp},
};

use super::{DataObjectHandle, OprType, Whence};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn make_seek_opened_data_obj_inp(
        handle: DataObjectHandle,
        whence: Whence,
        offset: usize,
    ) -> OpenedDataObjInp {
        OpenedDataObjInp::new(handle, 0, whence, OprType::No, offset, 0)
    }

    pub async fn seek(
        &mut self,
        handle: DataObjectHandle,
        whence: Whence,
        offset: usize,
    ) -> Result<&mut Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &Self::make_seek_opened_data_obj_inp(handle, whence, offset),
                MsgType::RodsApiReq,
                APN::DataObjLSeek as i32,
            )
            .and_then(|resc| resc.get_header_and_msg::<T, FileLseekOut>())
            .await?;

        Ok(self)
    }
}
