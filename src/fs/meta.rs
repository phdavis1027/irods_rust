use std::path::Path;

use futures::Stream;

use crate::{
    bosd::ProtocolEncoding, connection::Connection, error::errors::IrodsError, AccessControl, AVU,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn list_avus_for_data_object(
        &mut self,
        path: &Path,
    ) -> impl Stream<Item = Result<AccessControl, IrodsError>> {
        todo!()
    }

    pub async fn list_avus_for_collection(
        &mut self,
        path: &Path,
    ) -> impl Stream<Item = Result<AccessControl, IrodsError>> {
        todo!()
    }

    pub async fn add_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn remove_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        todo!()
    }
}
