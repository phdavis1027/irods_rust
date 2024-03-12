use std::path::Path;

use crate::{bosd::ProtocolEncoding, connection::Connection, error::errors::IrodsError};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    /// Should only be called on a data object
    pub async fn trim(&mut self, path: &Path) -> Result<(), IrodsError> {
        todo!()
    }

    /// Should only be called on a data object
    pub async fn repl(&mut self, path: &Path) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn add_child_resc(
        &mut self,
        parent: String,
        child: String,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn rm_child_resc(&mut self, parent: String, child: String) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn get_resource(&mut self, resc_name: String) -> Result<(), IrodsError> {
        todo!()
    }
}
