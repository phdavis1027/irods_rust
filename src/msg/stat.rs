use quick_xml::{events::Event, Reader};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{xml::XMLDeserializable, Deserializable},
    common::ObjectType,
};

#[derive(Debug)]
pub struct RodsObjStat {
    pub size: usize,
    pub object_type: ObjectType,
    pub mode: u32,
    pub id: u32,
    pub checksum: Option<u32>,
    pub owner_name: String,
    pub owner_zone: String,
    pub create_time: u64,
    pub modify_time: u64,
}

impl Deserializable for RodsObjStat {}
impl XMLDeserializable for RodsObjStat {
    fn from_xml(xml: &[u8]) -> Result<Self, rods_prot_msg::error::errors::IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            Size,
            SizeInner,
            ObjectType,
            ObjectTypeInner,
            Mode,
            ModeInner,
            Id,
            IdInner,
            Checksum,
            ChecksumInner,
            OwnerName,
            OwnerNameInner,
            OwnerZone,
            OwnerZoneInner,
            CreateTime,
            CreateTimeInner,
            ModifyTime,
            ModifyTimeInner,
        }

        let mut size: Option<usize> = None;
        let mut object_type: Option<ObjectType> = None;
        let mut mode: Option<u32> = None;
        let mut id: Option<u32> = None;
        let mut checksum: Option<u32> = None;
        let mut owner_name: Option<String> = None;
        let mut owner_zone: Option<String> = None;
        let mut create_time: Option<u64> = None;
        let mut modify_time: Option<u64> = None;

        let mut reader = Reader::from_reader(xml);

        let mut state = State::Tag;

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"RodsObjStat_PI" => {
                    State::Size
                }
                (State::Size, Event::Start(e)) if e.name().as_ref() == b"objSize" => {
                    State::SizeInner
                }
                (State::SizeInner, Event::Text(e)) => {
                    size = Some(e.unescape()?.parse()?);
                    State::ObjectType
                }
                (State::ObjectType, Event::Start(e)) if e.name().as_ref() == b"objType" => {
                    dbg!(&e);
                    State::ObjectTypeInner
                }
                (State::ObjectTypeInner, Event::Text(e)) => {
                    object_type = Some(match e.unescape()?.parse::<u32>()? {
                        0 => ObjectType::UnknownObj,
                        1 => ObjectType::DataObj,
                        2 => ObjectType::Coll,
                        3 => ObjectType::UnknownFile,
                        4 => ObjectType::LocalFile,
                        5 => ObjectType::LocalDir,
                        6 => ObjectType::NoInput,
                        _ => return Err(IrodsError::Other("Invalid value for ObjectType".into())),
                    });
                    State::Mode
                }
                (State::Mode, Event::Start(e)) if e.name().as_ref() == b"dataMode" => {
                    State::ModeInner
                }
                (State::ModeInner, Event::Text(e)) => {
                    mode = Some(e.unescape()?.parse()?);
                    State::Id
                }
                (State::Id, Event::Start(e)) if e.name().as_ref() == b"dataId" => State::IdInner,
                (State::IdInner, Event::Text(e)) => {
                    id = Some(e.unescape()?.parse()?);
                    State::Checksum
                }
                (State::Checksum, Event::Empty(e)) if e.name().as_ref() == b"chksum" => {
                    State::OwnerName
                }
                (State::Checksum, Event::Start(e)) if e.name().as_ref() == b"chksum" => {
                    State::ChecksumInner
                }
                (State::ChecksumInner, Event::Text(e)) => {
                    match e.unescape()?.parse() {
                        Ok(v) => checksum = Some(v),
                        Err(_) => checksum = None,
                    };
                    State::OwnerName
                }
                (State::OwnerName, Event::Start(e)) if e.name().as_ref() == b"ownerName" => {
                    State::OwnerNameInner
                }
                (State::OwnerNameInner, Event::Text(e)) => {
                    owner_name = Some(e.unescape()?.to_string());
                    State::OwnerZone
                }
                (State::OwnerZone, Event::Start(e)) if e.name().as_ref() == b"ownerZone" => {
                    State::OwnerZoneInner
                }
                (State::OwnerZoneInner, Event::Text(e)) => {
                    owner_zone = Some(e.unescape()?.to_string());
                    State::CreateTime
                }
                (State::CreateTime, Event::Start(e)) if e.name().as_ref() == b"createTime" => {
                    State::CreateTimeInner
                }
                (State::CreateTimeInner, Event::Text(e)) => {
                    create_time = Some(e.unescape()?.parse()?);
                    State::ModifyTime
                }
                (State::ModifyTime, Event::Start(e)) if e.name().as_ref() == b"modifyTime" => {
                    State::ModifyTimeInner
                }
                (State::ModifyTimeInner, Event::Text(e)) => {
                    modify_time = Some(e.unescape()?.parse()?);
                    return Ok(RodsObjStat {
                        size: size.ok_or(IrodsError::Other("Missing size".into()))?,
                        object_type: object_type
                            .ok_or(IrodsError::Other("Missing object type".into()))?,
                        mode: mode.ok_or(IrodsError::Other("Missing mode".into()))?,
                        id: id.ok_or(IrodsError::Other("Missing id".into()))?,
                        checksum,
                        owner_name: owner_name
                            .ok_or(IrodsError::Other("Missing owner name".into()))?,
                        owner_zone: owner_zone
                            .ok_or(IrodsError::Other("Missing owner zone".into()))?,
                        create_time: create_time
                            .ok_or(IrodsError::Other("Missing create time".into()))?,
                        modify_time: modify_time
                            .ok_or(IrodsError::Other("Missing modify time".into()))?,
                    });
                }
                (_, Event::Eof) => {
                    return Err(IrodsError::Other("Unexpected EOF".into()));
                }
                state => state.0,
            }
        }
    }
}
