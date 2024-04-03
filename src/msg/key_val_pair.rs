use std::{collections::HashMap, fmt::Debug, io::Write};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{xml::BorrowingXMLSerializable, BorrowingSerializable},
    common::kw::KeyWord,
    tag, tag_fmt,
};

#[cfg_attr(test, derive(Debug))]
pub struct BorrowingStrStrMap<'s> {
    vals: Vec<(KeyWord, &'s str)>,
}

impl<'s> BorrowingStrStrMap<'s> {
    pub fn new() -> Self {
        Self { vals: Vec::new() }
    }

    pub fn insert(&mut self, key: KeyWord, value: &'s str) {
        self.vals.push((key, value));
    }
}

impl<'s> BorrowingSerializable<'s> for BorrowingStrStrMap<'s> {}
impl<'s> BorrowingXMLSerializable<'s> for BorrowingStrStrMap<'s> {
    fn borrowing_xml_serialize<'r>(self, sink: &'r mut Vec<u8>) -> Result<usize, IrodsError>
    where
        's: 'r,
    {
        let mut cursor = std::io::Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("KeyValPair_PI")))?;

        tag_fmt!(writer, "ssLen", "{}", self.vals.len());

        for (key, _) in self.vals.iter() {
            tag!(writer, "keyWord", key.into());
        }

        writer.write_event(Event::End(BytesEnd::new("KeyValPair_PI")))?;

        Ok(cursor.position() as usize)
    }
}
