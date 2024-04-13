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
    fn make_read_data_obj_inp(handle: DataObjectHandle, len: usize) -> OpenedDataObjInp {
        OpenedDataObjInp::new(handle, len, Whence::SeekSet, OprType::No, 0, 0)
    }

    pub async fn read_data_obj_into(
        &mut self,
        handle: DataObjectHandle,
        buf: &mut Vec<u8>,
    ) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_read_data_obj_inp(handle, buf.capacity()),
                MsgType::RodsApiReq,
                APN::DataObjRead as i32,
            )
            .await?;

        let header = self.resources.read_standard_header::<T>().await?;

        self.resources
            .read_into_buf(buf, header.bs_len as usize)
            .await?;

        Ok(())
    }

    pub async fn read_data_obj_into_bytes_buf(
        &mut self,
        handle: DataObjectHandle,
        len: usize,
    ) -> Result<(), IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &Self::make_read_data_obj_inp(handle, len),
                MsgType::RodsApiReq,
                APN::DataObjRead as i32,
            )
            .await?;

        let (header, _) = self
            .resources
            .get_header_and_msg::<T, FileLseekOut>()
            .await?;

        self.resources
            .read_to_bytes_buf(header.bs_len as usize)
            .await?;

        Ok(())
    }
}
