use std::{collections::HashMap, fmt::Debug, io::Write};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{xml::BorrowingXMLSerializable, BorrowingSerializable},
    tag, tag_fmt,
};

#[cfg(not(test))]
impl<'s> BorrowingSerializable<'s> for HashMap<&'s str, &'s str> {}

#[cfg(test)]
impl<'s> BorrowingSerializable<'s> for HashMap<&'s str, &'s str> {}

impl<'s> BorrowingXMLSerializable<'s> for HashMap<&'s str, &'s str> {
    fn borrowing_xml_serialize<'r>(
        self,
        mut sink: &'r mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError>
    where
        's: 'r,
    {
        let mut cursor = std::io::Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("KeyValPair_PI")))?;

        tag_fmt!(writer, "ssLen", "{}", self.len());

        self.keys().try_for_each(|k| {
            tag!(writer, "keyWord", k);
            Ok::<_, IrodsError>(())
        })?;

        self.values().try_for_each(|v| {
            tag!(writer, "svalue", v);
            Ok::<_, IrodsError>(())
        })?;

        writer.write_event(Event::End(BytesEnd::new("KeyValPair_PI")))?;

        Ok(cursor.position() as usize)
    }
}

