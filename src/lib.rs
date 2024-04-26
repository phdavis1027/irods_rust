pub mod bosd;
pub mod common;
pub mod connection;
pub mod error;
pub mod fs;
pub mod gen_query;
pub mod msg;

pub mod exec_rule;
use std::path::{Path, PathBuf};
use std::time::Instant;

use chrono::{Date, DateTime, NaiveDateTime, Utc};
use common::icat_column::IcatColumn;
use error::errors::IrodsError;
pub use exec_rule_macro;
pub use exec_rule_macro::rule;
use gen_query::Row;

pub mod reexports {
    pub use derive_builder;
    pub use quick_xml;
    pub use tokio;
}

#[derive(Debug)]
pub enum DataObjectType {
    Generic,
    Tar,
    GzipTar,
    Bzip2,
    Zip,
    Msso,
}

impl TryFrom<&str> for DataObjectType {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "generic" => Ok(DataObjectType::Generic),
            "tar file" => Ok(DataObjectType::Tar),
            "gzipTar" => Ok(DataObjectType::GzipTar),
            "bzip2Tar" => Ok(DataObjectType::Bzip2),
            "zipFile" => Ok(DataObjectType::Zip),
            "msso file" => Ok(DataObjectType::Msso),
            _ => Err(IrodsError::Other("Invalid DataObjectType".to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct DataObject {
    id: i64,
    collection_id: i64,
    path: PathBuf,
    size: usize,
    data_type: DataObjectType,
    replica: ReplicaInfo,
}

impl DataObject {
    pub fn try_from_row_and_collection(
        value: &mut Row,
        coll_path: &Path,
    ) -> Result<Self, IrodsError> {
        let mut path = PathBuf::new();

        path.push(coll_path);

        let name = value
            .take(IcatColumn::DataObjectBaseName)
            .ok_or_else(|| IrodsError::Other("Missing name".to_owned()))?;

        path.push(name);

        Ok(Self {
            path,

            data_type: value
                .at(IcatColumn::DataObjectTypeName)
                .ok_or_else(|| IrodsError::Other("Missing data_type".to_owned()))?
                .as_str()
                .try_into()?,

            id: value
                .at(IcatColumn::DataObjectId)
                .ok_or_else(|| IrodsError::Other("Missing id".to_owned()))?
                .parse()
                .map_err(|_| IrodsError::Other("Failed to parse id".to_owned()))?,

            collection_id: value
                .at(IcatColumn::DataObjectCollectionId)
                .ok_or_else(|| IrodsError::Other("Missing collection_id".to_owned()))?
                .parse()
                .map_err(|_| IrodsError::Other("Failed to parse collection_id".to_owned()))?,

            size: value
                .at(IcatColumn::DataObjectSize)
                .ok_or_else(|| IrodsError::Other("Missing size".to_owned()))?
                .parse()
                .map_err(|_| IrodsError::Other("Failed to parse size".to_owned()))?,

            replica: ReplicaInfo::try_from(value)?,
        })
    }
}

//TODO: Should this be generic over timezone?
#[derive(Debug)]
pub struct ReplicaInfo {
    physical_path: String,
    id: i64,
    status: ReplStatus,
    resc_name: String,
    create_time: DateTime<Utc>,
    modify_time: DateTime<Utc>,
    resc_hierarchy: String,
    checksum: Option<String>,
}

impl TryFrom<&mut Row> for ReplicaInfo {
    type Error = IrodsError;

    fn try_from(value: &mut Row) -> Result<Self, Self::Error> {
        Ok(Self {
            resc_name: value
                .at(IcatColumn::DataObjectResourceName)
                .ok_or_else(|| IrodsError::Other("Missing resc_name".to_owned()))?
                .to_owned(),

            create_time: irods_instant(
                value
                    .at(IcatColumn::DataObjectCreateTime)
                    .ok_or_else(|| IrodsError::Other("Missing create_time".to_owned()))?,
            )?,

            modify_time: irods_instant(
                value
                    .at(IcatColumn::DataObjectModifyTime)
                    .ok_or_else(|| IrodsError::Other("Missing modify_time".to_owned()))?,
            )?,

            id: value
                .at(IcatColumn::DataObjectId)
                .ok_or_else(|| IrodsError::Other("Missing id".to_owned()))?
                .parse()
                .map_err(|_| IrodsError::Other("Failed to parse id".to_owned()))?,

            status: value
                .at(IcatColumn::DataObjectReplicastatus)
                .ok_or_else(|| IrodsError::Other("Missing status".to_owned()))?
                .as_str()
                .try_into()?,

            resc_hierarchy: value
                .take(IcatColumn::DataObjectResourceHierarchy)
                .ok_or_else(|| IrodsError::Other("Missing resc_hierarchy".to_owned()))?,

            physical_path: value
                .take(IcatColumn::DataObjectPhysicalPath)
                .ok_or_else(|| IrodsError::Other("Missing physical_path".to_owned()))?,

            checksum: value
                .at_mut(IcatColumn::DataObjectChecksum)
                .map(std::mem::take),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplStatus {
    Stale = 0,
    Good = 1,
    Intermediate = 2,
    ReadLocked = 3,
    WriteLocked = 4,
}

impl TryFrom<&str> for ReplStatus {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "0" => Ok(ReplStatus::Stale),
            "1" => Ok(ReplStatus::Good),
            "2" => Ok(ReplStatus::Intermediate),
            "3" => Ok(ReplStatus::ReadLocked),
            "4" => Ok(ReplStatus::WriteLocked),
            _ => Err(IrodsError::Other("Invalid ReplStatus".to_owned())),
        }
    }
}

#[derive(Debug)]
pub struct Collection {
    id: i64,
    path: PathBuf,
    owner: String,
    create_time: DateTime<Utc>,
    modify_time: DateTime<Utc>,
}

impl Collection {
    pub fn try_from_row_and_parent_collection(
        value: &mut Row,
        parent_path: &Path,
    ) -> Result<Self, IrodsError> {
        let mut path = PathBuf::new();

        path.push(parent_path);

        let name = value
            .take(IcatColumn::CollectionName)
            .ok_or_else(|| IrodsError::Other("Missing name".to_owned()))?;

        path.push(name);

        Ok(Self {
            path,

            id: value
                .at(IcatColumn::CollectionId)
                .ok_or_else(|| IrodsError::Other("Missing id".to_owned()))?
                .parse()
                .map_err(|_| IrodsError::Other("Failed to parse id".to_owned()))?,

            owner: value
                .take(IcatColumn::CollectionOwnerName)
                .ok_or_else(|| IrodsError::Other("Missing owner".to_owned()))?,

            create_time: irods_instant(
                value
                    .at(IcatColumn::CollectionCreateTime)
                    .ok_or_else(|| IrodsError::Other("Missing create_time".to_owned()))?,
            )?,

            modify_time: irods_instant(
                value
                    .at(IcatColumn::CollectionModifyTime)
                    .ok_or_else(|| IrodsError::Other("Missing modify_time".to_owned()))?,
            )?,
        })
    }
}

pub fn irods_instant(time: &str) -> Result<DateTime<Utc>, IrodsError> {
    let stamp = time
        .parse::<i64>()
        .map_err(|_| IrodsError::Other("Failed to parse timeestamp".to_owned()))?;

    Ok(DateTime::<Utc>::from_timestamp(stamp, 0).unwrap())
}
