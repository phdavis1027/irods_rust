use std::borrow::Cow;
use std::io::Cursor;

use quick_xml::events::BytesEnd;
use quick_xml::events::BytesStart;
use quick_xml::events::BytesText;
use quick_xml::events::Event;
use quick_xml::Writer;
use rods_prot_msg::error::errors::IrodsError;

use std::io::Write;

use crate::bosd::xml::BorrowingXMLDeserializable;
use crate::bosd::BorrowingDeserializable;
use crate::bosd::{xml::BorrowingXMLSerializable, BorrowingSerializable};
use crate::tag;
use crate::tag_fmt;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct BorrowingStrBuf<'s> {
    pub buf: Cow<'s, str>,
}

impl<'s> BorrowingStrBuf<'s> {
    pub fn new(buf: &'s str) -> Self {
        BorrowingStrBuf {
            buf: Cow::Borrowed(buf),
        }
    }
}

impl<'s> BorrowingSerializable<'s> for BorrowingStrBuf<'s> {}
impl<'s> BorrowingXMLSerializable<'s> for BorrowingStrBuf<'s> {
    fn borrowing_xml_serialize<'r>(
        self,
        sink: &'r mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError>
    where
        Self: Sized,
        's: 'r,
    {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("BinBytesBuf_PI")))?;

        tag_fmt!(writer, "bufLen", "{}", self.buf.len());

        tag!(writer, "buf", &self.buf);

        writer.write_event(Event::End(BytesEnd::new("BinBytesBuf_PI")))?;

        Ok(cursor.position() as usize)
    }
}

impl<'r> BorrowingDeserializable<'r> for BorrowingStrBuf<'r> {}
impl<'r> BorrowingXMLDeserializable<'r> for BorrowingStrBuf<'r> {
    fn borrowing_xml_deserialize<'s>(
        source: &'s [u8],
    ) -> Result<Self, rods_prot_msg::error::errors::IrodsError>
    where
        Self: Sized,
        's: 'r,
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
        let mut reader = quick_xml::Reader::from_reader(source);
        let mut state = State::Tag;

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(ref e)) if e.name().as_ref() == b"BinBytesBuf_PI" => {
                    State::BufLen
                }
                (State::BufLen, Event::Start(ref e)) if e.name().as_ref() == b"bufLen" => {
                    State::BufLenInner
                }
                (State::BufLenInner, Event::Text(_)) => {
                    // We don't actually care about the buf len
                    State::Buf
                }
                (State::Buf, Event::Start(ref e)) if e.name().as_ref() == b"buf" => State::BufInner,
                (State::BufInner, Event::Text(ref e)) => {
                    return Ok(BorrowingStrBuf { buf: e.unescape()? });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bosd::xml::XML;
    use crate::bosd::{BorrowingDeserializer, BorrowingSerializer};

    #[test]
    fn test_borrowing_str_buf_deserializes_correctly() {
        let expected = BorrowingStrBuf::new("hello world");

        let src = r#"<BinBytesBuf_PI><bufLen>11</bufLen><buf>hello world</buf></BinBytesBuf_PI>"#;
        let result: BorrowingStrBuf = XML::rods_borrowing_de(src.as_bytes()).unwrap();

        assert_eq!(expected, result);
    }

    #[test]
    fn test_borrowing_str_buf_serializes_correctly() {
        let expected =
            r#"<BinBytesBuf_PI><bufLen>11</bufLen><buf>hello world</buf></BinBytesBuf_PI>"#;

        let src = BorrowingStrBuf::new("hello world");
        let mut sink = Vec::new();
        XML::rods_borrowing_ser(src, &mut sink).unwrap();

        assert_eq!(expected.as_bytes(), sink.as_slice());
    }
}
