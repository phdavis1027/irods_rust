use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use rods_prot_msg::error::errors::IrodsError;
use serde::de::value;

use crate::{
    bosd::xml::BorrowingXMLSerializableChild, common::cond_input_kw::CondInputKw, tag, tag_fmt,
};

#[cfg_attr(test, derive(Debug))]
pub struct BorrowingCondInput<'s> {
    kw_map: Vec<(CondInputKw, &'s str)>,
}

impl<'s> BorrowingCondInput<'s> {
    pub fn new() -> BorrowingCondInput<'s> {
        BorrowingCondInput { kw_map: Vec::new() }
    }

    pub fn add_kw(&mut self, kw: CondInputKw, val: &'s str) {
        self.kw_map.push((kw, val));
    }

    pub fn get_kw(&self, kw: CondInputKw) -> Option<&'s str> {
        for (key, value) in self.kw_map.iter() {
            if *key == kw {
                return Some(value);
            }
        }
        None
    }

    pub fn set_kw(&mut self, kw: CondInputKw) {
        self.kw_map.push((kw, ""));
    }
}

impl<'s> BorrowingXMLSerializableChild<'s> for BorrowingCondInput<'s> {
    fn borrowing_xml_serialize_child<'r, 't1, 't2>(
        self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError>
    where
        's: 'r,
        's: 't1,
        's: 't2,
    {
        writer.write_event(Event::Start(BytesStart::new("KeyValPair_PI")))?;

        tag_fmt!(writer, "ssLen", "{}", self.kw_map.len());

        for (key, _) in self.kw_map.iter() {
            tag!(writer, "keyWord", key.into());
        }

        for (_, value) in self.kw_map {
            tag!(writer, "svalue", value);
        }

        writer.write_event(Event::End(BytesEnd::new("KeyValPair_PI")))?;

        Ok(())
    }
}
