use std::{fs::Metadata, path::Path};

use crate::{
    bosd::ProtocolEncoding,
    common::ObjectType,
    connection::{authenticate::Authenticate, connect::Connect, pool::ConnectionPool},
    error::errors::IrodsError,
};

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
                self.do_upload_file_task(local_path, remote_path, meta, 0)
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

    pub async fn do_upload_file_task(
        &mut self,
        local_path: &Path,
        remote_path: &Path,
        meta: Metadata,
        offset: u64,
    ) -> Result<(), IrodsError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".into()))?;

        Ok(())
    }

    pub async fn upload_file_parallel(
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
