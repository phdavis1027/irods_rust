use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    error::errors::IrodsError,
    msg::{header::MsgType, opened_data_obj_inp::OpenedDataObjInp},
};

use super::{DataObjectHandle, OprType, Whence};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn make_write_data_obj_inp(handle: DataObjectHandle, len: usize) -> OpenedDataObjInp {
        OpenedDataObjInp::new(handle, len, Whence::SeekSet, OprType::No, 0, 0)
    }

    pub async fn write_data_obj_from_bytes_buf(
        &mut self,
        handle: DataObjectHandle,
    ) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_write_data_obj_inp(handle, buf.len()),
                MsgType::RodsApiReq,
                APN::DataObjWrite as i32,
            )
            .await?;

        tokio::io::copy(&mut buf, &mut self.resources.transport).await?;

        Ok(())
    }
}
