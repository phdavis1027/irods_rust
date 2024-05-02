use std::path::Path;

use crate::{
    bosd::ProtocolEncoding,
    common::{AccessLevel, ObjectType, UserType, APN},
    connection::{Connection, MAX_PASSWORD_LEN},
    error::errors::IrodsError,
    msg::{acls::ModifyAccessRequest, admin::GeneralAdminInpBuilder, header::MsgType},
    AdminOperation, AdminTarget,
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
    pub async fn set_access_inherit(
        &mut self,
        path: &Path,
        recursive: bool,
        inherit: bool,
    ) -> Result<(), IrodsError> {
        let stat = self.stat(path).await?;
        let ObjectType::Coll = stat.object_type else {
            return Err(IrodsError::Other("Path is not a collection".to_string()));
        };

        let access_level = match inherit {
            true => AccessLevel::Inherit,
            false => AccessLevel::NoInherit,
        };
        let inp = ModifyAccessRequest::new(
            recursive,
            access_level,
            String::new(),
            String::new(),
            path.to_path_buf(),
        );

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::ModAccessControl as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn add_user_to_group(
        &mut self,
        user: String,
        group: String,
        zone: String,
    ) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Modify)
            .target(AdminTarget::Group)
            .two(group)
            .three("add".to_owned())
            .four(user)
            .five(zone)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn remove_user_from_group(
        &mut self,
        user: String,
        group: String,
        zone: String,
    ) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Modify)
            .target(AdminTarget::Group)
            .two(group)
            .three("remove".to_owned())
            .four(user)
            .five(zone)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn create_group(&mut self, group: String, ty: String) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Add)
            .target(AdminTarget::User)
            .two(group)
            .three(ty)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn change_user_type(
        &mut self,
        user: String,
        user_type: UserType,
        zone: String,
    ) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Modify)
            .target(AdminTarget::User)
            .two(format!("{}#{}", user, zone))
            .three("type".to_owned())
            .four(Into::<&str>::into(user_type).to_string())
            .five(zone)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn delete_user(&mut self, user: String, zone: String) -> Result<(), IrodsError> {
        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Remove)
            .target(AdminTarget::User)
            .two(user)
            .three(zone)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }

    pub async fn change_user_password(
        &mut self,
        user: String,
        zone: String,
        password: String,
    ) -> Result<(), IrodsError> {
        if password.is_empty() {
            return Err(IrodsError::Other("Password cannot be empty".to_string()));
        }

        if password.len() > MAX_PASSWORD_LEN {
            return Err(IrodsError::Other(
                "Password cannot be longer than 50 characters".to_string(),
            ));
        }

        let mut len_copy = MAX_PASSWORD_LEN - 10 - password.len();

        let inp = GeneralAdminInpBuilder::default()
            .action(AdminOperation::Modify)
            .target(AdminTarget::User)
            .two(user)
            .three("password".to_owned())
            .five(zone)
            .build()
            .unwrap();

        Ok(())
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
            .action(AdminOperation::Add)
            .target(AdminTarget::SpecQuery)
            .two(query)
            .build()
            .unwrap();

        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GeneralAdmin as i32)
            .await?;

        let _ = self.resources.read_standard_header::<T>().await?;

        Ok(())
    }
}
