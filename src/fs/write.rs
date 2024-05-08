use std::io::Cursor;

use tokio::io::AsyncReadExt;

use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    error::errors::IrodsError,
    msg::{header::MsgType, opened_data_obj_inp::OpenedDataObjInp},
};

use super::{DataObjectHandle, OprType, Whence};

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
        len: usize,
    ) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_write_data_obj_inp(handle, self.resources.bytes_buf.len()),
                MsgType::RodsApiReq,
                APN::DataObjWrite as i32,
            )
            .await?;

        tokio::io::copy(
            &mut Cursor::new(&mut self.resources.bytes_buf).take(len as u64),
            &mut self.resources.transport,
        )
        .await?;

        self.resources.read_standard_header::<T>().await?;

        Ok(())
    }
}
