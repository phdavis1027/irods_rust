use std::path::Path;

use futures::{pin_mut, StreamExt};

use crate::{
    bosd::ProtocolEncoding,
    common::icat_column::IcatColumn,
    connection::Connection,
    error::errors::IrodsError,
    irods_instant,
    msg::gen_query::{IcatPredicate, QueryBuilder},
    DataObject,
};

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub async fn ls_data_objects(&mut self, path: &Path) -> Result<Vec<DataObject>, IrodsError> {
        let mut inp = QueryBuilder::new()
            .select(IcatColumn::DataObjectId)
            .select(IcatColumn::DataObjectBaseName)
            .select(IcatColumn::DataObjectSize)
            .select(IcatColumn::DataObjectTypeName)
            .select(IcatColumn::DataObjectReplNum)
            .select(IcatColumn::DataObjectOwnerName)
            .select(IcatColumn::DataObjectChecksum)
            .select(IcatColumn::DataObjectReplicastatus)
            .select(IcatColumn::DataObjectRescourceName)
            .select(IcatColumn::DataObjectPhysicalPath)
            .select(IcatColumn::DataObjectResourceHierarchy)
            .select(IcatColumn::DataObjectCreateTime)
            .select(IcatColumn::DataObjectModifyTime)
            .select(IcatColumn::DataObjectReplicastatus)
            .condition(
                IcatColumn::CollectionName,
                IcatPredicate::Equals(path.to_str().unwrap().to_owned()),
            )
            .build();

        let out = self.query(&mut inp).await;
        pin_mut!(out);

        while let Some(row) = out.next().await {
            println!("{:?}", row);
            let row = row?;

            let modify_time =
                irods_instant(row.at(IcatColumn::DataObjectModifyTime).unwrap().as_str())?;

            let create_time =
                irods_instant(row.at(IcatColumn::DataObjectCreateTime).unwrap().as_str())?;

            println!("{:?}", row.at(IcatColumn::DataObjectReplicastatus).unwrap());
        }

        unimplemented!()
    }
}
