use std::{
    marker::PhantomPinned,
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future::BoxFuture, stream, FutureExt, Stream, StreamExt, TryStreamExt};

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

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn one_off_query(&mut self, inp: &GenQueryInp) -> Result<GenQueryOut, IrodsError> {
        self.send_header_then_msg(inp, MsgType::RodsApiReq, APN::GenQuery as i32)
            .await?;

        let (_, out) = self.get_header_and_msg::<GenQueryOut>().await?;

        Ok(out)
    }

    pub async fn query(
        &mut self,
        inp: &GenQueryInp,
    ) -> impl Stream<Item = Result<Option<Vec<String>>, IrodsError>> {
        futures::stream::try_unfold((), |_| async move {
            let page = self.one_off_query(inp).await?.into_page_of_rows();
            let stream = stream::iter(page.iter());
            Ok(Some((stream, ())))
        })
        .try_flatten()
    }
}

impl GenQueryOut {
    pub fn into_page_of_rows(self) -> Vec<Vec<String>> {
        let mut rows = Vec::new();

        for cur_row_idx in 0..self.row_count as usize {
            let mut row = Vec::with_capacity(self.attr_count as usize);

            for col in self.columns.iter_mut().take(self.attr_count as usize) {
                let val = std::mem::take(&mut col.value);
                row.push(val);
            }

            rows.push(row);
        }

        rows
    }
}

/*
pub struct Query<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    conn: &'conn mut Connection<T, C>,
    inp: GenQueryInp,
    last_page: bool,
    rows_processed: u32,
    current_page: Option<Vec<Vec<String>>>,
    query_future: Option<BoxFuture<'static, Result<GenQueryOut, IrodsError>>>,
    _pinned: PhantomPinned,
}

impl<'conn, T, C> Query<'conn, T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(conn: &'conn mut Connection<T, C>, inp: GenQueryInp) -> Self {
        Self {
            conn,
            inp,
            last_page: false,
            rows_processed: 0,
            current_page: None,
            query_future: None,
            _pinned: PhantomPinned,
        }
    }
}

impl<'conn, T, C> Stream for Query<'conn, T, C>
where
    T: ProtocolEncoding + Send + Sync,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync,
{
    type Item = Result<Vec<String>, IrodsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { Pin::into_inner_unchecked(self) };

        if this.current_page.is_some() {
            // Do we have a page ready?
            let page = unsafe { this.current_page.as_mut().unwrap_unchecked() };
            match page.pop() {
                // Does this page have another row?
                Some(row) => {
                    this.rows_processed += 1;
                    return Poll::Ready(Some(Ok(row)));
                }
                None => {
                    // If not, start polling for another page
                    this.current_page = None;
                }
            }
        }

        if this.query_future.is_none() {
            if this.last_page {
                return Poll::Ready(None);
            }
            if this.rows_processed >= this.inp.max_rows {
                return Poll::Ready(None);
            }

            let fut: BoxFuture<Result<GenQueryOut, IrodsError>> =
                this.conn.one_off_query(&this.inp).boxed();

            let fut: BoxFuture<'static, Result<GenQueryOut, IrodsError>> =
                unsafe { std::mem::transmute(fut) };

            this.query_future = Some(fut);
        }

        match unsafe { this.query_future.as_mut().unwrap_unchecked().poll_unpin(cx) } {
            Poll::Ready(out_result) => match out_result {
                Ok(out) => {
                    let mut page: Vec<Vec<String>> = out.into_page_of_rows();

                    this.query_future = None;

                    match page.pop() {
                        Some(row) => {
                            this.rows_processed += 1;

                            return Poll::Ready(Some(Ok(row)));
                        }
                        None => {
                            this.current_page = None;
                        }
                    }
                }
                Err(err) => return Poll::Ready(Some(Err(err))),
            },
            Poll::Pending => return Poll::Pending,
        };

        Poll::Ready(None)
    }
}
*/
