use quick_xml::events::{BytesEnd, BytesStart, Event};
use rods_prot_msg::{error::errors::IrodsError, types::MsgType};
use std::io::{Cursor, Write};

use crate::{
    bosd::{
        xml::{OwningXMLDeserializable, OwningXMLSerializable},
        OwningDeserializble, OwningSerializable,
    },
    common::{CsNegPolicy, CsNegResult},
    tag_fmt,
};

#[cfg_attr(test, derive(Debug))]
pub struct OwningServerCsNeg {
    pub status: i32,
    pub result: CsNegPolicy,
}

impl OwningServerCsNeg {
    pub fn new(status: i32, result: CsNegPolicy) -> Self {
        Self { status, result }
    }
}

impl OwningDeserializble for OwningServerCsNeg {}
impl OwningXMLDeserializable for OwningServerCsNeg {
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

#[cfg_attr(test, derive(Debug))]
pub struct OwningClientCsNeg {
    pub status: i32,
    pub result: CsNegResult,
}

impl OwningClientCsNeg {
    pub fn new(status: i32, result: CsNegResult) -> Self {
        Self { status, result }
    }
}

impl OwningSerializable for OwningClientCsNeg {}

impl OwningXMLSerializable for OwningClientCsNeg {
    fn owning_xml_serialize(&self, buf: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(buf);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("CS_NEG_PI")))?;

        tag_fmt!(writer, "status", "{}", self.status);

        let result: &str = (&self.result).into();
        tag_fmt!(writer, "result", "cs_neg_result_kw={}", result);

        writer.write_event(Event::End(BytesEnd::new("CS_NEG_PI")))?;

        Ok(cursor.position() as usize)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::common::CsNegResult;

    #[test]
    fn client_cs_neg_serialize_correctly() {
        let cs_neg = OwningClientCsNeg::new(0, CsNegResult::CS_NEG_USE_SSL);

        let mut buf = Vec::new();
        cs_neg.owning_xml_serialize(&mut buf).unwrap();

        let expected = r#"<CS_NEG_PI><status>0</status><result>cs_neg_result_kw=CS_NEG_USE_SSL</result></CS_NEG_PI>"#;
        assert_eq!(String::from_utf8(buf).unwrap(), expected);
    }

    #[test]
    fn server_cs_neg_deserialize_correctly() {
        let src = r#"<CS_NEG_PI><status>0</status><result>CS_NEG_REFUSE</result></CS_NEG_PI>"#;
        let cs_neg = OwningServerCsNeg::owning_xml_deserialize(src.as_bytes()).unwrap();

        assert_eq!(cs_neg.status, 0);
        assert_eq!(cs_neg.result, CsNegPolicy::CS_NEG_REFUSE);
    }
}
