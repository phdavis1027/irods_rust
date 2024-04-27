use std::path::Path;

use async_stream::try_stream;
use futures::{pin_mut, Stream};

use crate::{
    bosd::ProtocolEncoding,
    common::icat_column::IcatColumn,
    connection::Connection,
    error::errors::IrodsError,
    msg::gen_query::{IcatPredicate, QueryBuilder},
    AccessControl, AVU,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn list_avus_for_data_object(
        &mut self,
        path: &Path,
    ) -> impl Stream<Item = Result<AccessControl, IrodsError>> {
        todo!()
    }

    pub async fn list_avus_for_collection<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<AVU, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let inp = QueryBuilder::new()
            .select(IcatColumn::MetadataAttributeId)
            .select(IcatColumn::MetadataAttributeName)
            .select(IcatColumn::MetadataAttributeValue)
            .select(IcatColumn::MetadataAttributeUnits)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_string()),
            )
            .build();

        let mut stream = self.query(&mut inp).await;

        pin_mut!(stream);

        try_stream! {
            for await row in stream {
                let row = row?;
                yield AVU::try_from_row(&mut row)?
            }
        }
    }

    pub async fn add_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        todo!()
    }

    pub async fn remove_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        todo!()
    }
}
