use irods_xml::events::{BytesEnd, BytesStart, Event};
use crate::error::errors::IrodsError;
use std::io::{Cursor, Read, Write};

use crate::{
    bosd::{
        xml::{XMLDeserializable, XMLSerializable},
        Deserializable, Serialiazable,
    },
    common::{CsNegPolicy, CsNegResult},
    tag_fmt,
};

#[derive(Debug)]
pub struct ServerCsNeg {
    pub status: i32,
    pub result: CsNegPolicy,
}

impl ServerCsNeg {
    pub fn new(status: i32, result: CsNegPolicy) -> Self {
        Self { status, result }
    }
}

impl Deserializable for ServerCsNeg {}
impl XMLDeserializable for ServerCsNeg {
    fn from_xml(xml: &[u8]) -> Result<Self, crate::error::errors::IrodsError>
    where
        Self: Sized,
    {
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

        let mut reader = irods_xml::Reader::from_reader(xml);
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

#[derive(Debug)]
pub struct ClientCsNeg {
    pub status: i32,
    pub result: CsNegResult,
}

impl ClientCsNeg {
    pub fn new(status: i32, result: CsNegResult) -> Self {
        Self { status, result }
    }
}

impl Serialiazable for ClientCsNeg {}
impl XMLSerializable for ClientCsNeg {
    fn to_xml(
        &self,
        sink: &mut Vec<u8>,
    ) -> Result<usize, crate::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = irods_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("CS_NEG_PI")))?;

        tag_fmt!(writer, "status", "{}", self.status);

        let result: &str = (&self.result).into();
        tag_fmt!(writer, "result", "cs_neg_result_kw={}", result);

        writer.write_event(Event::End(BytesEnd::new("CS_NEG_PI")))?;

        Ok(cursor.position() as usize)
    }
}
