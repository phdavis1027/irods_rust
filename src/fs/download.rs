use std::{os::unix::fs::FileExt, path::Path};

use deadpool::managed::{self, Object};
use futures::{stream::FuturesUnordered, StreamExt};
use rods_prot_msg::error::errors::IrodsError;
use tokio::fs::OpenOptions;

use crate::{
    bosd::ProtocolEncoding,
    common::ObjectType,
    connection::{authenticate::Authenticate, connect::Connect, pool::ConnectionPool, Connection},
};

use super::OpenFlag;

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
        }
    }

    pub fn force_overwrite(&mut self) -> &mut Self {
        self.force_overwrite = true;
        self
    }

    pub fn on_resource(&mut self, resource: String) -> &mut Self {
        self.resource = Some(resource);
        self
    }

    pub async fn download(self) -> Result<(), IrodsError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

        let stat = conn.stat(self.remote_path).await?;

        match stat.object_type {
            ObjectType::UnknownObj => {
                Err(IrodsError::Other("Path does not exist in zone".to_string()))
            }
            _ if self.local_path.exists() && !self.force_overwrite => Err(IrodsError::Other(
                "Local path exists and force_overwrite flag is not set".to_string(),
            )),
            ObjectType::DataObj if self.local_path.exists() => {
                tokio::fs::remove_file(self.local_path).await?;
                self.download_data_object_parallel(stat.size).await
            }
            ObjectType::DataObj => self.download_data_object_parallel(stat.size).await,
            ObjectType::Coll if self.local_path.exists() => {
                tokio::fs::remove_dir_all(self.local_path).await?;
                Ok(())
            }
            ObjectType::Coll if self.recursive => Ok(()),
            ObjectType::Coll => Err(IrodsError::Other(
                "Collection download without recursive flag".to_string(),
            )),
            _ => Err(IrodsError::Other("Invalid local path".to_string())),
        }
    }

    pub async fn download_data_object_parallel(self, size: usize) -> Result<(), IrodsError> {
        let len_per_task = ((size as f64 / self.num_tasks as f64).floor() as usize)
            + ((((size as u32) % self.num_tasks != 0) as usize) as usize);

        dbg!(len_per_task);

        let futs = FuturesUnordered::new();
        for task in 0..self.num_tasks {
            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|_| IrodsError::Other("Failed to get connection".to_string()))?;

            let resource = self.resource.clone();

            futs.push(async move {
                conn.do_parallel_download_task(
                    self.remote_path,
                    self.local_path,
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
        println!("Downloading task {} of size {}", task, len);
        println!("Opening local file for writing at path: {:?}", local_path);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(local_path)
            .await?;

        println!("Opening data object for reading");
        let handle = self.open_request(remote_path).execute().await?;

        let mut offset = task * len;

        println!("Seeked to offset {}", offset);
        if offset > 0 {
            self.seek(handle, super::Whence::SeekSet, offset).await?;
        }
        let mut buf = Vec::with_capacity(len);
        println!("Reading data object into buffer");
        self.read_data_obj_into(handle, &mut buf).await?;

        let file = file.into_std().await;
        tokio::task::spawn_blocking(move || {
            println!("Writing to file at offset {}", offset);
            file.write_all_at(&buf, offset as u64)
        })
        .await
        .map_err(|_| IrodsError::Other("Failed to write to file".to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    use crate::{
        bosd::xml::XML,
        connection::{
            authenticate::NativeAuthenticator, pool::IrodsManager, tcp::TcpConnector, Account,
        },
    };

    use super::*;

    #[tokio::test]
    async fn test_parallel_download() {
        let account = Account::test_account();

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(172, 18, 0, 3), 1247));
        let connector = TcpConnector::new(addr);
        let authenticator = NativeAuthenticator::new(30, "rods".into());
        let manager: IrodsManager<XML, TcpConnector, NativeAuthenticator> =
            IrodsManager::new(account, connector, authenticator, 10, 10);

        let mut pool: managed::Pool<IrodsManager<_, _, _>> = managed::Pool::builder(manager)
            .max_size(30)
            .build()
            .unwrap();

        ParallelDownloadContext::new(
            &mut pool,
            10,
            &Path::new("/tempZone/home/rods/totc.txt"),
            &Path::new("./totc.txt"),
        )
        .download()
        .await
        .unwrap();
    }
}
