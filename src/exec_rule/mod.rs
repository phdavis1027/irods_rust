use std::net::SocketAddr;

use crate::{
    bosd::{Deserializable, ProtocolEncoding, Serialiazable},
    connection::Connection,
};

pub mod exec_rule_out;

pub struct RuleExecContext<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    instance: Option<String>,
    addr: Option<SocketAddr>,
    conn: &'conn mut Connection<T, C>,
}

pub trait Rule {
    type Output: Deserializable;

    async fn execute<'conn, T, C>(self, ctx: &mut RuleExecContext<'conn, T, C>) -> Self::Output
    where
        T: ProtocolEncoding,
        C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin;
}

impl<'conn, T, C> RuleExecContext<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    fn new(conn: &'conn mut Connection<T, C>) -> Self {
        Self {
            instance: None,
            addr: None,
            conn,
        }
    }

    fn instance(mut self, instance: String) -> Self {
        self.instance = Some(instance);
        self
    }

    fn addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }
}

#[derive(Debug)]
pub struct ExecRuleOut {
    pub std_out: String,
    pub std_err: String,
    pub exit_code: i32,
}
