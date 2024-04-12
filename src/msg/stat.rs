use quick_xml::{events::Event, Reader};

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

impl RodsObjStat {
    pub fn new(
        size: usize,
        object_type: ObjectType,
        mode: u32,
        id: u32,
        checksum: Option<u32>,
        owner_name: String,
        owner_zone: String,
        create_time: u64,
        modify_time: u64,
    ) -> Self {
        Self {
            size,
            object_type,
            mode,
            id,
            checksum,
            owner_name,
            owner_zone,
            create_time,
            modify_time,
        }
    }
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
                    State::Size,
                }
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"objSize" => State::SizeInner,
                (State::SizeInner, Event::Text(e)) => {
                    size = Some(e.unescape()?.parse()?);
                    State::ObjectType 
                }
                (State::ObjectType, Event::Start(e)) if e.name().as_ref() == b"objType" => State::ObjectTypeInner,
                (State::ObjectTypeInner, Event::Text(e)) => {
                    object_type = Some(e.unescape()?.parse()?);
                    State::Mode
                }
            }
        }

    }
}
