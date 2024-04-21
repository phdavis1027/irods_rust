use crate::{
    bosd::ProtocolEncoding,
    common::{IrodsProt, APN},
    connection::Connection,
    error::errors::IrodsError,
    msg::{
        gen_query::{GenQueryInp, GenQueryOut},
        header::MsgType,
    },
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(crate) async fn query(&mut self, inp: GenQueryInp) -> Result<GenQueryOut, IrodsError> {
        self.send_header_then_msg(&inp, MsgType::RodsApiReq, APN::GenQuery as i32)
            .await?;

        let (_, out) = self.get_header_and_msg::<GenQueryOut>().await?;

        Ok(out)
    }
}
