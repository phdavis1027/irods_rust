use deadpool::managed;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::ProtocolEncoding,
    connection::{authenticate::Authenticate, connect::Connect, pool::IrodsManager, Connection},
};

use super::{DataObjectHandle, Whence};

pub async fn download_data_object_parallel<T, C, A>(
    pool: managed::Pool<IrodsManager<T, C, A>>,
    local_path: String,
    remote_path: String,
    remote_resource: String,
    num_tasks: usize,
    data_object_len: usize,
) -> Result<(), IrodsError>
where
    T: ProtocolEncoding + Send + Sync,
    C: Connect<T> + Send + Sync + 'static,
    C::Transport: Send + Sync + 'static,
    A: Authenticate<T, C::Transport> + Send + Sync + 'static,
{
    Ok(())
}
