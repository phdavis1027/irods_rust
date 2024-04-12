use rods_prot_msg::error::errors::IrodsError;

use crate::{bosd::ProtocolEncoding, connection::Connection};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
}
