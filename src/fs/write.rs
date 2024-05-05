use crate::error::errors::IrodsError;

use super::DataObjectHandle;

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn write_data_obj_from_buf(
        &mut self,
        handle: DataObjectHandle,
        buf: &mut [u8],
    ) -> Result<(), IrodsError> {
    }
}
