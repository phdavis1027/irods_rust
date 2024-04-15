use crate::{bosd::ProtocolEncoding, connection::Connection};

pub mod exec_rule_out;

pub struct RuleExecRequest<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    instance: Option<String>,
    conn: &'conn mut Connection<T, C>,
}

impl<'conn, T, C> RuleExecRequest<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn new(conn: &'conn mut Connection<T, C>) -> Self {
        Self {
            instance: None,
            conn,
        }
    }

    fn with_instance(mut self, instance: String) -> Self {
        self.instance = Some(instance);
        self
    }
}
