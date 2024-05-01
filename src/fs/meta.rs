use std::path::Path;

use async_stream::try_stream;
use futures::{pin_mut, Stream};

use crate::{
    bosd::ProtocolEncoding,
    common::{icat_column::IcatColumn, ObjectType},
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
    pub async fn list_avus_for_data_object<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<AVU, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::MetadataAttributeId)
            .select(IcatColumn::MetadataAttributeName)
            .select(IcatColumn::MetadataAttributeValue)
            .select(IcatColumn::MetadataAttributeUnits)
            .condition(
                IcatColumn::DataObjectBaseName,
                IcatPredicate::Equals(path.file_name().unwrap().to_str().unwrap().to_string()),
            )
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.parent().unwrap().to_str().unwrap().to_string()),
            )
            .build();

        try_stream! {
            let stream = self.query(&mut inp).await;
            for await row in stream {
                let mut row = row?;
                yield AVU::try_from_row(&mut row)?;
            }
        }
    }

    pub async fn list_avus_for_collection<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<AVU, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::MetadataAttributeId)
            .select(IcatColumn::MetadataAttributeName)
            .select(IcatColumn::MetadataAttributeValue)
            .select(IcatColumn::MetadataAttributeUnits)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_string()),
            )
            .build();

        try_stream! {
            let stream = self.query(&mut inp).await;
            for await row in stream {
                let mut row = row?;
                yield AVU::try_from_row(&mut row)?;
            }
        }
    }

    pub async fn add_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        let stat = self.stat(path).await?;

        match stat.object_type {
            ObjectType::Coll => 
        }
    }

    pub async fn remove_avu(&mut self, path: &Path, avu: &AVU) -> Result<(), IrodsError> {
        todo!()
    }
}
