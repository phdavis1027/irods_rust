use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    bosd::{Deserializable, ProtocolEncoding},
    connection::Connection,
};

use crate::error::errors::IrodsError;

pub mod exec_rule_out;

pub trait Rule {
    type Output: Deserializable;

    async fn execute<'c, T, C>(
        &self,
        conn: &'c mut Connection<T, C>,
    ) -> Result<Self::Output, IrodsError>
    where
        T: ProtocolEncoding,
        C: AsyncRead + AsyncWrite + Unpin + Send;
}

#[derive(Debug)]
pub enum ExecRuleOut {
    Some {
        std_out: String,
        std_err: String,
        exit_code: i32,
    },
    None,
}
