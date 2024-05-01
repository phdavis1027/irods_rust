use std::path::Path;

use async_stream::try_stream;
use futures::{pin_mut, Stream, StreamExt, TryFuture};

use crate::{
    bosd::ProtocolEncoding,
    common::icat_column::IcatColumn,
    connection::Connection,
    error::errors::IrodsError,
    irods_instant,
    msg::gen_query::{IcatPredicate, QueryBuilder},
    Collection, DataObject, ReplicaInfo, AVU,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn ls_data_objects<'this, 'p>(
        &'this mut self,
        path: &'p Path,
        max_results: u32,
        master_replica: bool,
        avu_filter: Option<&'p AVU>,
    ) -> impl Stream<Item = Result<DataObject, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let inp = QueryBuilder::new()
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
            .max_rows(max_results);

        let inp = match avu_filter {
            Some(avu) => inp
                .condition(
                    IcatColumn::MetadataAttributeName,
                    IcatPredicate::Equals(avu.attribute.to_string()),
                )
                .condition(
                    IcatColumn::MetadataAttributeValue,
                    IcatPredicate::Equals(avu.value.to_string()),
                ),
            None => inp,
        };

        let mut inp = match master_replica {
            true => inp
                .condition(
                    IcatColumn::DataObjectReplNum,
                    IcatPredicate::Equals("1".to_string()),
                )
                .build(),
            false => inp.build(),
        };

        // this is the fastest way I can think of to avoid
        // fighting with the stream combinators
        try_stream! {
            for await row in self.query(&mut inp).await {
                let mut row = row?;
                yield DataObject::try_from_row_and_collection(&mut row, path)?;
            }
        }
    }

    pub async fn ls_sub_collections<'this, 'p>(
        &'this mut self,
        path: &'p Path,
        max_results: u32,
    ) -> impl Stream<Item = Result<Collection, IrodsError>> + 'this
    where
        'p: 'this,
    {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::CollectionId)
            .select(IcatColumn::CollectionName)
            .select(IcatColumn::CollectionOwnerName)
            .select(IcatColumn::CollectionModifyTime)
            .select(IcatColumn::CollectionCreateTime)
            .condition(
                IcatColumn::CollectionParentName,
                IcatPredicate::Equals(path.to_str().unwrap().to_owned()),
            )
            .max_rows(max_results)
            .build();

        try_stream! {
            for await row in self.query(&mut inp).await {
                let mut row = row?;
                yield Collection::try_from_row_and_parent_collection(&mut row, &path)?;
            }
        }
    }
}
