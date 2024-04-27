use std::{io::Write, os::unix::fs::FileExt, path::Path};

use crate::{error::errors::IrodsError, msg::stat::RodsObjStat};
use futures::{future::BoxFuture, pin_mut, stream::FuturesUnordered, FutureExt, StreamExt};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::{
    bosd::ProtocolEncoding,
    common::ObjectType,
    connection::{authenticate::Authenticate, connect::Connect, pool::ConnectionPool, Connection},
};

pub struct ParallelDownloadContext<'pool, 'path, T, C, A>
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
    create: bool,
    recursive: bool,
    max_collection_children: u32,
    max_size_before_parallel: usize,
}

impl<'pool, 'path, T, C, A> ParallelDownloadContext<'pool, 'path, T, C, A>
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
            force_overwrite: false,
            recursive: false,
            create: true,
            resource: None,
            max_collection_children: 500,
            max_size_before_parallel: 32 * (1024_usize.pow(2)), // Default from PRC
        }
    }

    pub fn recursive(&mut self) -> &mut Self {
        self.recursive = true;
        self
    }

    pub fn force_overwrite(&mut self) -> &mut Self {
        self.force_overwrite = true;
        self
    }

    pub fn on_resource(&mut self, resource: String) -> &mut Self {
        self.resource = Some(resource);
        self
    }

    pub fn max_collection_children(&mut self, max: u32) -> &mut Self {
        self.max_collection_children = max;
        self
    }

    pub fn max_size_before_parallel(&mut self, max: usize) -> &mut Self {
        self.max_size_before_parallel = max;
        self
    }

    pub async fn download(mut self) -> Result<(), IrodsError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

        let stat = conn.stat(self.remote_path).await?;

        match stat.object_type {
            _ if self.local_path.exists() && !self.force_overwrite => Err(IrodsError::Other(
                "Local path exists and force_overwrite flag is not set".to_string(),
            )),
            ObjectType::UnknownObj => {
                Err(IrodsError::Other("Path does not exist in zone".to_string()))
            }
            ObjectType::DataObj => {
                let remote_path = self.remote_path;
                let local_path = self.local_path;

                self.download_data_object(&stat, &remote_path, &local_path)
                    .await
            }
            ObjectType::Coll if !self.recursive => Err(IrodsError::Other(
                "Collection download without recursive flag".to_string(),
            )),
            ObjectType::Coll => {
                let local_path = self.local_path;
                let remote_path = self.remote_path;

                self.download_collection(&remote_path, &local_path).await
            }
            _ => Err(IrodsError::Other("Invalid path".to_string())),
        }
    }

    pub async fn download_data_object(
        &mut self,
        stat: &RodsObjStat,
        src: &Path,
        dst: &Path,
    ) -> Result<(), IrodsError> {
        if stat.size > self.max_size_before_parallel {
            self.download_data_object_parallel(stat.size as usize).await
        } else {
            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(dst)
                .await?;

            let handle = conn.open_request(src).execute().await?;

            conn.read_data_obj_into_bytes_buf(handle, stat.size as usize)
                .await?;

            let size = stat.size as usize;

            let mut file = file.into_std().await;

            let mut buf = std::mem::take(&mut conn.resources.bytes_buf);

            let buf = tokio::task::spawn_blocking(move || {
                file.write_all(&mut buf[..size])?;
                file.sync_all()?;
                Ok::<_, IrodsError>(buf)
            })
            .await
            .map_err(|_| IrodsError::Other("Failed to write to file".to_string()))??;

            conn.resources.bytes_buf = buf;

            conn.close(handle).await?;

            Ok(())
        }
    }

    pub async fn stat_and_download_data_object(
        &mut self,
        src: &Path,
        dst: &Path,
    ) -> Result<(), IrodsError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

        let stat = conn.stat(src).await?;

        self.download_data_object(&stat, src, dst).await
    }

    pub fn download_collection<'this, 'd>(
        &'this mut self,
        src: &'d Path,
        dst: &'d Path,
    ) -> BoxFuture<'this, Result<(), IrodsError>>
    // Boxing and sync signature needed to
    // A) prevent the async statement from
    // compiling a cycle on recursion
    // B) running afowl of the borrow checker, i.e.,
    // we can manually specify the lifetimes
    where
        'd: 'this,
    {
        async move {
            if self.local_path.exists() {
                tokio::fs::remove_dir_all(dst).await?
            }
            tokio::fs::create_dir(self.local_path).await?;

            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

            let data_objects = conn
                .ls_data_objects(self.remote_path, self.max_collection_children)
                .await;

            pin_mut!(data_objects);

            while let Some(data_object) = data_objects.next().await {
                let data_object = data_object?;
                let remote_path = src.join(data_object.path.file_name().unwrap());
                let local_path = dst.join(data_object.path.file_name().unwrap());

                self.stat_and_download_data_object(&remote_path, &local_path)
                    .await?;
            }

            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

            let sub_collections = conn
                .ls_sub_collections(self.remote_path, self.max_collection_children)
                .await;

            pin_mut!(sub_collections);

            while let Some(sub_collection) = sub_collections.next().await {
                println!("Subcollection: {:?}", sub_collection);
                let sub_collection = sub_collection?;

                let remote_path = src.join(sub_collection.path.file_name().unwrap());
                let local_path = dst.join(sub_collection.path.file_name().unwrap());

                self.download_collection(&remote_path, &local_path).await?;
            }

            Ok(())
        }
        .boxed()
    }

    pub async fn download_data_object_parallel(&mut self, size: usize) -> Result<(), IrodsError> {
        let len_per_task = ((size as f64 / self.num_tasks as f64).floor() as usize)
            + ((((size as u32) % self.num_tasks != 0) as usize) as usize);

        let futs = FuturesUnordered::new();
        for task in 0..self.num_tasks {
            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

            let resource = self.resource.clone();
            let remote_path = self.remote_path.to_path_buf();
            let local_path = self.local_path.to_path_buf();

            futs.push(async move {
                conn.do_parallel_download_task(
                    &remote_path,
                    &local_path,
                    resource,
                    task as usize,
                    len_per_task,
                )
                .await?;

                Ok::<_, IrodsError>(())
            });
        }

        futs.collect::<Vec<_>>().await;
        Ok(())
    }
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding + Send,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    async fn do_parallel_download_task(
        &mut self,
        remote_path: &Path,
        local_path: &Path,
        resource: Option<String>,
        task: usize,
        len: usize,
    ) -> Result<(), IrodsError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(local_path)
            .await?;

        let handle = self.open_request(remote_path).execute().await?;

        let offset = task * len;

        if offset > 0 {
            self.seek(handle, super::Whence::SeekSet, offset).await?;
        }

        // let mut buf = Vec::with_capacity(len);
        self.read_data_obj_into_bytes_buf(handle, len).await?;

        // self.read_data_obj_into(handle, &mut buf).await?;
        let file = file.into_std().await;

        // This is a hack to get around the borrow checker
        // Anything that enters the tokio::task::spawn_blocking closure
        // must be static. However, we don't want to allocate another buffer.
        // We have exclusive access to `self`, so we know that nobody will use
        // it in the meantime, so we can just take it and give it back after
        // tokio is done with it. This prevents extra allocations because Vec
        // doesn't allocate until something is pushed to it.
        let mut buf = std::mem::take(&mut self.resources.bytes_buf);
        let buf = tokio::task::spawn_blocking(move || {
            file.write_all_at(&mut buf[..len], offset as u64)?;
            file.sync_all()?;
            Ok::<_, IrodsError>(buf)
        })
        .await
        .map_err(|_| IrodsError::Other("Failed to write to file".to_string()))??;

        self.resources.bytes_buf = buf;

        self.close(handle).await?;

        Ok(())
    }
}
