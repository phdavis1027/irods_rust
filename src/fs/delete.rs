use std::path::Path;

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn delete(&mut self, path: &Path) -> Result<(), Error> {
        Ok(())
    }
}
