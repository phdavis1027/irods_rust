use quick_xml::events::Event;
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{xml::OwningXMLDeserializable, OwningDeserializble},
    common::CsNegPolicy,
};

pub struct OwningCsNeg {
    pub status: i32,
    pub result: CsNegPolicy,
}

impl OwningCsNeg {
    pub fn new(status: i32, result: CsNegPolicy) -> Self {
        Self { status, result }
    }
}

impl OwningDeserializble for OwningCsNeg {}
impl OwningXMLDeserializable for OwningCsNeg {
    fn owning_xml_deserialize(src: &[u8]) -> Result<Self, IrodsError> {
        #[derive(Debug)]
        #[repr(u8)]
        enum State {
            Tag,
            Status,
            StatusInner,
            Result,
            ResultInner,
        }

        let mut status: Option<i32> = None;
        let mut result: Option<CsNegPolicy> = None;

        let mut state = State::Tag;

        let mut reader = quick_xml::Reader::from_reader(src);
        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(ref e)) if e.name().as_ref() == b"CS_NEG_PI" => {
                    State::Status
                }
                (State::Status, Event::Start(ref e)) if e.name().as_ref() == b"status" => {
                    State::StatusInner
                }
                (State::StatusInner, Event::Text(text)) => {
                    status = Some(text.unescape()?.as_ref().parse()?);
                    State::Result
                }
                (State::Result, Event::Start(ref e)) if e.name().as_ref() == b"result" => {
                    State::ResultInner
                }
                (State::ResultInner, Event::Text(text)) => {
                    result = Some(text.unescape()?.as_ref().try_into()?);

                    return Ok(Self::new(
                        status.ok_or(IrodsError::Other("status not found".into()))?,
                        result.ok_or(IrodsError::Other("result not found".into()))?,
                    ));
                }
                (_, Event::Eof) => {
                    return Err(IrodsError::Other("unexpected EOF".into()));
                }
                state => state.0,
            }
        }
    }
}
