use async_stream::try_stream;

use crate::{
    bosd::ProtocolEncoding,
    common::{icat_column::IcatColumn, APN},
    connection::Connection,
    error::errors::IrodsError,
    msg::{
        gen_query::{GenQueryInp, GenQueryOut},
        header::MsgType,
    },
};

use futures::{self, Stream};

#[derive(Debug)]
pub struct Row(Vec<(IcatColumn, String)>);

impl Row {
    pub fn at<'this>(&'this self, col: IcatColumn) -> Option<&'this String> {
        self.0.iter().find(|(k, _)| *k == col).map(|(_, v)| v)
    }

    pub fn at_mut<'this>(&'this mut self, col: IcatColumn) -> Option<&'this mut String> {
        self.0.iter_mut().find(|(k, _)| *k == col).map(|(_, v)| v)
    }

    pub fn take(&mut self, col: IcatColumn) -> Option<String> {
        match self.0.iter_mut().find(|(k, _)| *k == col) {
            Some((_, v)) => Some(std::mem::take(v)),
            None => None,
        }
    }
}

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

    // TODO: Reimplement this in terms of futures::stream::(try_)unfold
    // and futures::stream::StreamExt::flatten. My assumption is that
    // since these don't use message passing, they should be more efficient.
    pub async fn query<'this, 'inp>(
        &'this mut self,
        inp: &'inp mut GenQueryInp,
    ) -> impl Stream<Item = Result<Row, IrodsError>> + 'this
    where
        'inp: 'this,
    {
        try_stream! {
            let mut more_pages = true;
            let mut rows_processed = 0;
            while more_pages {
                let out = self.one_off_query(inp).await?;

                inp.continue_index = out.continue_index;

                more_pages = out.continue_index > 0;

                let page = out.into_page_of_rows();

                for await row in page {
                    if (inp.max_rows != 0) && (rows_processed >= inp.max_rows) {
                        more_pages = false;
                        break;
                    }

                    rows_processed += 1;
                    yield row;
                }
            }
        }
    }
}

impl GenQueryOut {
    pub fn into_page_of_rows(mut self) -> impl Stream<Item = Row> {
        let mut rows = match self.columns.get(0) {
            Some(column) => {
                let mut rows = Vec::with_capacity(column.1.len());
                for _ in 0..column.1.len() {
                    rows.push(Row(Vec::new()));
                }
                rows
            }
            None => {
                return futures::stream::iter(Vec::new());
            }
        };

        for (cat_inx, col) in self.columns.iter_mut() {
            for (row_inx, val) in col.iter_mut().enumerate() {
                rows[row_inx].0.push((cat_inx.clone(), std::mem::take(val)));
            }
        }

        futures::stream::iter(rows)
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
