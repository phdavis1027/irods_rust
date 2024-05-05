use std::{
    fs::{Metadata, OpenOptions},
    os::unix::fs::FileExt,
    path::Path,
};

use crate::{
    bosd::ProtocolEncoding,
    common::ObjectType,
    connection::{authenticate::Authenticate, connect::Connect, pool::ConnectionPool, Connection},
    error::errors::IrodsError,
};

use super::OpenFlag;

pub struct ParallelTransferContext<'pool, 'path, T, C, A>
where
    T: ProtocolEncoding + Send + Sync,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    pool: &'pool mut ConnectionPool<T, C, A>,
    num_tasks: u32,
    remote_path: &'path Path,
    local_path: &'path Path,
    resource: Option<String>,
    force_overwrite: bool,
    recursive: bool,
    max_size_before_parallel: usize,
}

impl<'pool, 'path, T, C, A> ParallelTransferContext<'pool, 'path, T, C, A>
where
    T: ProtocolEncoding + Send + Sync,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    pub fn new(
        pool: &'pool mut ConnectionPool<T, C, A>,
        num_tasks: u32,
        remote_path: &'path Path,
        local_path: &'path Path,
    ) -> Self {
        Self {
            pool,
            num_tasks,
            remote_path,
            local_path,
            resource: None,
            force_overwrite: false,
            recursive: false,
            max_size_before_parallel: 32 * (1024_usize.pow(2)), // Default from PRC
        }
    }

    pub fn resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn force_overwrite(mut self) -> Self {
        self.force_overwrite = true;
        self
    }

    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    pub fn onto_resource(&mut self, resource: String) -> &mut Self {
        self.resource = Some(resource);
        self
    }

    pub fn max_size_before_parallel(&mut self, size: usize) -> &mut Self {
        self.max_size_before_parallel = size;
        self
    }

    pub async fn upload(mut self) -> Result<(), IrodsError> {
        let meta = self
            .local_path
            .metadata()
            .map_err(|_| IrodsError::Other("Failed to stat local path".into()))?;

        if meta.is_file() {
            self.upload_file(self.local_path, self.remote_path, meta)
                .await?;
        } else if meta.is_dir() && self.recursive {
            self.upload_dir(self.local_path, self.remote_path, meta)
                .await?;
        } else if meta.is_dir() {
            return Err(IrodsError::Other(
                "Path is a directory and recursive flag not set".into(),
            ));
        } else {
            return Err(IrodsError::Other("Path is not a file or directory".into()));
        }

        Ok(())
    }

    pub async fn upload_file(
        &mut self,
        local_path: &Path,
        remote_path: &Path,
        meta: Metadata,
    ) -> Result<(), IrodsError> {
        if meta.len() > self.max_size_before_parallel as u64 {
            return self
                .upload_file_parallel(local_path, remote_path, meta)
                .await;
        }

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".into()))?;

        let stat = conn.stat(self.remote_path).await?;
        match stat.object_type {
            ObjectType::UnknownObj => {
                conn.do_parallel_upload_task(
                    self.remote_path,
                    self.local_path,
                    self.resource.clone(),
                    0,
                    meta.len() as usize,
                )
                .await?
            }
            ObjectType::Coll if self.force_overwrite => {}
            ObjectType::DataObj if self.force_overwrite => {}
            _ => {
                return Err(IrodsError::Other(
                    "Remote path already exists and overwrite flag not set".into(),
                ));
            }
        }

        Ok(())
    }

    pub async fn upload_file_parallel(
        &mut self,
        local_path: &Path,
        remote_path: &Path,
        meta: Metadata,
    ) -> Result<(), IrodsError> {
        Ok(())
    }

    pub async fn upload_dir(
        &mut self,
        local_path: &Path,
        remote_path: &Path,
        meta: Metadata,
    ) -> Result<(), IrodsError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".into()))?;

        Ok(())
    }
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding + Send,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    async fn do_parallel_upload_task(
        &mut self,
        remote_path: &Path,
        local_path: &Path,
        resource: Option<String>,
        task: usize,
        len: usize,
    ) -> Result<(), IrodsError> {
        let file = OpenOptions::new().read(true).open(local_path)?;

        let handle = match resource {
            Some(resc) => {
                self.open_request(remote_path)
                    .set_resc(resc.as_str())
                    .set_flag(OpenFlag::WriteOnly)
                    .execute()
                    .await?
            }
            None => {
                self.open_request(remote_path)
                    .set_flag(OpenFlag::WriteOnly)
                    .execute()
                    .await?
            }
        };

        let mut buf = std::mem::take(&mut self.resources.bytes_buf);
        let buf = tokio::task::spawn_blocking(move || {
            let mut remaining = len;
            let mut offset = (task * len) as usize;
            loop {
                let read = file.read_at(&mut buf[..remaining], offset)?;
                // transfer n bytes
                self.self
                    .self
                    .seek(handle, super::Whence::SeekCur, read)
                    .await?;
            }

            Ok::<_, IrodsError>(buf)
        })
        .await
        .map_err(|_| IrodsError::Other("Failed to transfer file".to_string()))??;

        self.resources.bytes_buf = buf;

        self.close(handle).await?;

        Ok(())
    }
}
