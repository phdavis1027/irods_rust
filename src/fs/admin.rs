use std::path::Path;

use crate::{
    bosd::ProtocolEncoding,
    common::{AccessLevel, UserType, APN},
    connection::Connection,
    error::errors::IrodsError,
    msg::{admin::GeneralAdminInpBuilder, header::MsgType},
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    /// Stat the path, call appropriate method
    pub async fn chmod(
        &mut self,
        path: &Path,
        access_level: AccessLevel,
        user: String,
        zone: String,
        admin: bool,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    /// Should only be called on a collection
    pub async fn set_access_inherit(&mut self, path: &Path) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn add_user_to_group(
        &mut self,
        user: String,
        group: String,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn remove_user_from_group(
        &mut self,
        user: String,
        group: String,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn create_group(&mut self, group: String, ty: String) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn change_user_type(
        &mut self,
        user: String,
        user_type: UserType,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn delete_user(&mut self, user: String) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn change_user_password(
        &mut self,
        user: String,
        password: String,
    ) -> Result<(), IrodsError> {
        todo!()
    }

    /// GetGroup in go-irodsclient
    pub async fn list_users_in_group(&mut self, group: String) -> Result<Vec<String>, IrodsError> {
        todo!()
    }

    pub async fn list_groups(&mut self) -> Result<Vec<String>, IrodsError> {
        todo!()
    }

    pub async fn list_all_users(&mut self) -> Result<Vec<String>, IrodsError> {
        todo!()
    }

    pub async fn list_quotas_for_group(&mut self, group: String) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn list_quotas_for_user(&mut self, user: String) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn register_spec_query(&mut self, query: String) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .zero("add".to_string())
            .one("specQuery".to_string())
            .two(query)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }
}
