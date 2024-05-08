use std::path::Path;

use crate::{
    bosd::ProtocolEncoding,
    common::{cond_input_kw::CondInputKw, ObjectType},
    connection::Connection,
    error::errors::IrodsError,
    msg::data_obj_inp::DataObjInp,
};

use super::OprType;

pub struct DeleteRequest<'conn, 'p, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    conn: &'conn mut Connection<T, C>,
    path: &'p Path,
    force: bool,
    recursive: bool,
}

impl<'conn, 'p, T, C> DeleteRequest<'conn, 'p, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(conn: &'conn mut Connection<T, C>, path: &'p Path) -> Self {
        Self {
            conn,
            path,
            force: false,
            recursive: false,
        }
    }

    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    pub async fn execute(self) -> Result<(), IrodsError> {
        self.conn
            .delete(self.path, self.force, self.recursive)
            .await
    }
}

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn make_delete_data_object_request(path: &Path, force: bool) -> DataObjInp {
        let mut inp = DataObjInp::new(path.to_str().unwrap().to_owned(), OprType::No, 0, 0);

        if force {
            inp.cond_input.set_kw(CondInputKw::ForceFlagKw);
        }

        inp
    }

    async fn delete<'path>(
        &mut self,
        path: &'path Path,
        force: bool,
        recursive: bool,
    ) -> Result<(), IrodsError> {
        let stat = self.stat(path).await?;

        match stat.object_type {
            ObjectType::Coll => {
                self.delete_coll(path).await?;
            }
            ObjectType::DataObj => {
                self.delete_data_obj(path).await?;
            }
            _ => {
                return Err(IrodsError::Other("No such path".to_string()));
            }
        }

        Ok(())
    }

    async fn delete_coll<'path>(&mut self, path: &'path Path) -> Result<(), IrodsError> {
        Ok(())
    }

    async fn delete_data_obj<'path>(&mut self, path: &'path Path) -> Result<(), IrodsError> {
        Ok(())
    }
}
