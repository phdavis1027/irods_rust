use std::borrow::Cow;
use std::io::Cursor;

use irods_xml::events::BytesEnd;
use irods_xml::events::BytesStart;
use irods_xml::events::BytesText;
use irods_xml::events::Event;
use irods_xml::Writer;
use crate::error::errors::IrodsError;

use std::io::Write;

use crate::bosd::xml::XMLDeserializable;
use crate::bosd::xml::XMLSerializable;
use crate::bosd::Deserializable;
use crate::bosd::Serialiazable;
use crate::tag;
use crate::tag_fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct BinBytesBuf {
    pub buf: String,
}

impl BinBytesBuf {
    pub fn new(buf: &str) -> Self {
        BinBytesBuf {
            buf: String::from(buf),
        }
    }
}

impl Serialiazable for BinBytesBuf {}
impl XMLSerializable for BinBytesBuf {
    fn to_xml(&self, mut sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("BinBytesBuf_PI")))?;

        tag_fmt!(writer, "buflen", "{}", self.buf.len());
        tag!(writer, "buf", &self.buf);

        writer.write_event(Event::End(BytesEnd::new("BinBytesBuf_PI")))?;

        Ok(cursor.position() as usize)
    }
}

impl Deserializable for BinBytesBuf {}
impl XMLDeserializable for BinBytesBuf {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[derive(Debug)]
        #[repr(u8)]
        enum State {
            Tag,
            BufLen,
            BufLenInner,
            Buf,
            BufInner,
        }
        let mut reader = irods_xml::Reader::from_reader(xml);
        let mut state = State::Tag;

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(ref e)) if e.name().as_ref() == b"BinBytesBuf_PI" => {
                    State::BufLen
                }
                (State::BufLen, Event::Start(ref e)) if e.name().as_ref() == b"buflen" => {
                    State::BufLenInner
                }
                (State::BufLenInner, Event::Text(_)) => {
                    // We don't actually care about the buf len
                    State::Buf
                }
                (State::Buf, Event::Start(ref e)) if e.name().as_ref() == b"buf" => State::BufInner,
                (State::BufInner, Event::Text(ref e)) => {
                    return Ok(BinBytesBuf {
                        buf: e.unescape()?.to_string(),
                    });
                }
                (state, Event::Eof) => {
                    return Err(IrodsError::Other(format!(
                        "unexpected EOF in state: [{:?}]",
                        state
                    )))
                }
                (state, _) => state,
            };
        }
    }
}
