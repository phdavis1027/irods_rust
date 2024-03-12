use crate::error::errors::IrodsError;

use crate::bosd::ProtocolEncoding;

use super::{Account, ResourceBundle, UnauthenticatedConnection};

pub trait Connect<T>: Send
where
    T: ProtocolEncoding + Send,
{
    type Transport: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send;

    fn connect(
        &self,
        acct: Account,
    ) -> impl std::future::Future<
        Output = Result<UnauthenticatedConnection<T, Self::Transport>, IrodsError>,
    > + std::marker::Send;
}
