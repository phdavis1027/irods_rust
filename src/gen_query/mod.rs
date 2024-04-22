use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, Future, FutureExt, Stream};

use crate::{
    bosd::ProtocolEncoding,
    common::APN,
    connection::Connection,
    error::errors::IrodsError,
    msg::{
        gen_query::{GenQueryInp, GenQueryOut},
        header::MsgType,
    },
};

use pin_project::pin_project;

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    pub async fn query(&mut self, inp: &GenQueryInp) -> Result<GenQueryOut, IrodsError> {
        self.send_header_then_msg(inp, MsgType::RodsApiReq, APN::GenQuery as i32)
            .await?;

        let (_, out) = self.get_header_and_msg::<GenQueryOut>().await?;

        Ok(out)
    }
}

pub struct Query<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    conn: &'conn mut Connection<T, C>,
    inp: GenQueryInp,
    query_future: Option<BoxFuture<'static, Result<GenQueryOut, IrodsError>>>,
    _pinned: PhantomPinned,
}

impl<'conn, T, C> Stream for Query<'conn, T, C>
where
    T: ProtocolEncoding + Send + Sync,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Sync,
{
    type Item = Result<Vec<String>, IrodsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { Pin::into_inner_unchecked(self) };

        if this.query_future.is_none() {
            let future = this.conn.query(&this.inp).boxed();
        }
    }
}
