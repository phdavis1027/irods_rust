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
use common::{AccessLevel, ObjectType, UserType};
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

pub struct AccessControl {
    path: PathBuf,
    user_name: String,
    user_zone: String,
    user_type: UserType,
    access_type: AccessLevel,
}

impl AccessControl {
    pub fn try_from_row_and_path_for_data_object(
        row: &mut Row,
        path: &Path,
    ) -> Result<Self, IrodsError> {
        Ok(Self {
            path: path.to_owned(),
            user_name: row
                .take(IcatColumn::UserName)
                .ok_or_else(|| IrodsError::Other("Missing user_name".to_owned()))?,
            user_zone: row
                .take(IcatColumn::UserZone)
                .ok_or_else(|| IrodsError::Other("Missing user_zone".to_owned()))?,
            user_type: row
                .take(IcatColumn::UserType)
                .ok_or_else(|| IrodsError::Other("Missing user_type".to_owned()))?
                .as_str()
                .try_into()?,
            access_type: row
                .take(IcatColumn::DataObjectAccessName)
                .ok_or_else(|| IrodsError::Other("Missing access_type".to_owned()))?
                .as_str()
                .try_into()?,
        })
    }

    pub fn try_from_row_and_path_for_collection(
        row: &mut Row,
        path: &Path,
    ) -> Result<Self, IrodsError> {
        Ok(Self {
            path: path.to_owned(),
            user_name: row
                .take(IcatColumn::UserName)
                .ok_or_else(|| IrodsError::Other("Missing user_name".to_owned()))?,
            user_zone: row
                .take(IcatColumn::UserZone)
                .ok_or_else(|| IrodsError::Other("Missing user_zone".to_owned()))?,
            user_type: row
                .take(IcatColumn::UserType)
                .ok_or_else(|| IrodsError::Other("Missing user_type".to_owned()))?
                .as_str()
                .try_into()?,
            access_type: row
                .take(IcatColumn::CollectionAccessName)
                .ok_or_else(|| IrodsError::Other("Missing access_type".to_owned()))?
                .as_str()
                .try_into()?,
        })
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
    owner: String,
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
            owner: value
                .take(IcatColumn::DataObjectOwnerName)
                .ok_or_else(|| IrodsError::Other("Missing owner".to_owned()))?,
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

pub struct Entry {
    id: i64,
    entry_type: ObjectType,
    path: PathBuf,
    owner: String,
    size: usize,
    create_time: DateTime<Utc>,
    modify_time: DateTime<Utc>,
    data_type: Option<DataObjectType>,
    checksum: Vec<u8>,
    checksum_algo: ChecksumAlgo,
}

pub enum ChecksumAlgo {
    SHA1,
    SHA256,
    SHA512,
    ADLER32,
    MD5,
    Unknown,
}

impl From<&str> for ChecksumAlgo {
    fn from(value: &str) -> Self {
        match value {
            "SHA-1" => ChecksumAlgo::SHA1,
            "SHA-256" => ChecksumAlgo::SHA256,
            "SHA-512" => ChecksumAlgo::SHA512,
            "ADLER-32" => ChecksumAlgo::ADLER32,
            "MD5" => ChecksumAlgo::MD5,
            _ => ChecksumAlgo::Unknown,
        }
    }
}

impl From<Collection> for Entry {
    fn from(value: Collection) -> Self {
        Self {
            id: value.id,
            entry_type: ObjectType::Coll,
            path: value.path,
            owner: value.owner,
            size: 0,
            create_time: value.create_time,
            modify_time: value.modify_time,
            data_type: None,
            checksum: Vec::new(),
            checksum_algo: ChecksumAlgo::Unknown,
        }
    }
}

impl From<DataObject> for Entry {
    fn from(value: DataObject) -> Self {
        Self {
            id: value.id,
            entry_type: ObjectType::DataObj,
            path: value.path,
            owner: "".to_owned(),
            size: value.size,
            create_time: value.replica.create_time,
            modify_time: value.replica.modify_time,
            data_type: Some(value.data_type),
            checksum: Vec::new(),
            checksum_algo: ChecksumAlgo::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct AVU {
    id: i64,
    attribute: String,
    value: String,
    unit: String,
}

impl AVU {
    fn try_from_row(row: &mut Row) -> Result<Self, IrodsError> {
        Ok(Self {
            id: row
                .take(IcatColumn::MetadataAttributeId)
                .ok_or_else(|| IrodsError::Other("Missing id".to_owned()))?
                .parse()?,
            attribute: row
                .take(IcatColumn::MetadataAttributeName)
                .ok_or_else(|| IrodsError::Other("Missing attribute".to_owned()))?,

            value: row
                .take(IcatColumn::MetadataAttributeValue)
                .ok_or_else(|| IrodsError::Other("Missing value".to_owned()))?,

            unit: row
                .take(IcatColumn::MetadataAttributeUnits)
                .ok_or_else(|| IrodsError::Other("Missing unit".to_owned()))?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AVUTarget {
    User,
    Collection,
    DataObject,
    Resource,
}

impl Into<&str> for AVUTarget {
    fn into(self) -> &'static str {
        match self {
            AVUTarget::User => "-u",
            AVUTarget::Collection => "-C",
            AVUTarget::DataObject => "-u",
            AVUTarget::Resource => "-R",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AVUOperation {
    Add,
    AddWildcard,
    Modify,
    Copy,
    Remove,
    RemoveWildcard,
    RemoveById,
    Set,
}

impl Into<&str> for AVUOperation {
    fn into(self) -> &'static str {
        match self {
            AVUOperation::Add => "add",
            AVUOperation::AddWildcard => "addw",
            AVUOperation::Modify => "mod",
            AVUOperation::Copy => "cp",
            AVUOperation::Remove => "rm",
            AVUOperation::RemoveWildcard => "rmw",
            AVUOperation::RemoveById => "rmi",
            AVUOperation::Set => "set",
        }
    }
}
