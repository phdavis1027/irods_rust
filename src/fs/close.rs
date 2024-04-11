use futures::TryFutureExt;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    msg::{data_obj_inp::DataObjInp, header::MsgType, opened_data_obj_inp::OpenedDataObjInp},
};

use super::{DataObjectHandle, OprType, Whence};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncWrite + tokio::io::AsyncRead + Unpin,
{
    fn make_close_opened_data_obj_inp(handle: DataObjectHandle) -> OpenedDataObjInp {
        OpenedDataObjInp::new(handle, 0, Whence::SeekSet, OprType::No, 0, 0)
    }

    pub async fn close(&mut self, handle: DataObjectHandle) -> Result<&mut Self, IrodsError> {
        self.inner
            .resources
            .send_header_then_msg::<T, _>(
                &Self::make_close_opened_data_obj_inp(handle),
                MsgType::RodsApiReq,
                APN::DataObjClose as i32,
            )
            .and_then(|resc| resc.read_standard_header::<T>())
            .await?;

        Ok(self)
    }
}
