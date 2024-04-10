use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::ProtocolEncoding;

use super::{Account, ResourceBundle, UnauthenticatedConnection};

pub trait Connect<T>
where
    T: ProtocolEncoding,
{
    type Transport: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin;

    async fn connect(
        &self,
        acct: Account,
    ) -> Result<UnauthenticatedConnection<T, Self::Transport>, IrodsError>;
}
