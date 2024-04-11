use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use rods_prot_msg::error::errors::IrodsError;

use crate::{bosd::xml::XMLSerializableChild, common::cond_input_kw::CondInputKw, tag, tag_fmt};

#[derive(Debug)]
pub struct CondInput {
    kw_map: Vec<(CondInputKw, String)>,
}

impl CondInput {
    pub fn new() -> CondInput {
        CondInput { kw_map: Vec::new() }
    }

    pub fn add_kw(&mut self, kw: CondInputKw, val: String) {
        self.kw_map.push((kw, val));
    }

    pub fn get_kw(&self, kw: CondInputKw) -> Option<String> {
        for (key, value) in self.kw_map.iter() {
            if *key == kw {
                return Some(value.clone());
            }
        }

        None
    }

    pub fn set_kw(&mut self, kw: CondInputKw) {
        self.kw_map.push((kw, "".to_string()));
    }
}

impl XMLSerializableChild for CondInput {
    fn to_nested_xml<'r, 't1, 't2>(
        &self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError> {
        writer.write_event(Event::Start(BytesStart::new("KeyValPair_PI")))?;

        tag_fmt!(writer, "ssLen", "{}", self.kw_map.len());

        for (key, _) in self.kw_map.iter() {
            tag!(writer, "keyWord", key.into());
        }

        for (_, value) in self.kw_map.iter() {
            tag!(writer, "svalue", &value);
        }

        writer.write_event(Event::End(BytesEnd::new("KeyValPair_PI")))?;

        Ok(())
    }
}
