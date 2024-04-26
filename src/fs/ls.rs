use std::path::Path;

use async_stream::try_stream;
use futures::{pin_mut, Stream, StreamExt};

use crate::{
    bosd::ProtocolEncoding,
    common::icat_column::IcatColumn,
    connection::Connection,
    error::errors::IrodsError,
    irods_instant,
    msg::gen_query::{IcatPredicate, QueryBuilder},
    DataObject, ReplicaInfo,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn ls_data_objects<'this, 'p>(
        &'this mut self,
        path: &'p Path,
    ) -> impl Stream<Item = Result<DataObject, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::DataObjectId)
            .select(IcatColumn::DataObjectBaseName)
            .select(IcatColumn::DataObjectSize)
            .select(IcatColumn::DataObjectTypeName)
            .select(IcatColumn::DataObjectReplNum)
            .select(IcatColumn::DataObjectOwnerName)
            .select(IcatColumn::DataObjectChecksum)
            .select(IcatColumn::DataObjectReplicastatus)
            .select(IcatColumn::DataObjectResourceName)
            .select(IcatColumn::DataObjectPhysicalPath)
            .select(IcatColumn::DataObjectResourceHierarchy)
            .select(IcatColumn::DataObjectCreateTime)
            .select(IcatColumn::DataObjectModifyTime)
            .select(IcatColumn::DataObjectReplicastatus)
            .select(IcatColumn::DataObjectCollectionId)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_owned()),
            )
            .build();

        // this is the fastest way I can think of to avoid
        // fighting with the stream combinators
        try_stream! {
            for await row in self.query(&mut inp).await {
                let mut row = row?;
                yield DataObject::try_from_row_and_collection(&mut row, path)?;
            }
        }
    }
}
