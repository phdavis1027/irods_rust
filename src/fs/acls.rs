use std::path::Path;

use async_stream::try_stream;
use futures::{future::BoxFuture, Stream};

use crate::{
    bosd::ProtocolEncoding,
    common::{icat_column::IcatColumn, ObjectType},
    connection::Connection,
    error::errors::IrodsError,
    msg::gen_query::{IcatPredicate, QueryBuilder},
    AccessControl,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn list_acls_for_data_object<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<AccessControl, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::DataObjectAccessName)
            .select(IcatColumn::UserName)
            .select(IcatColumn::UserZone)
            .select(IcatColumn::UserType)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_string()),
            )
            .build();

        try_stream! {
            for await row in self.query(&mut inp).await {
                let mut row = row?;
                yield AccessControl::try_from_row_and_path_for_data_object(&mut row, path)?;
            }
        }
    }

    pub async fn list_acls_for_collection<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<AccessControl, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::CollectionAccessName)
            .select(IcatColumn::UserName)
            .select(IcatColumn::UserZone)
            .select(IcatColumn::UserType)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_string()),
            )
            .build();

        try_stream! {
            for await row in self.query(&mut inp).await {
                let mut row = row?;
                yield AccessControl::try_from_row_and_path_for_collection(&mut row, path)?;
            }
        }
    }
}
