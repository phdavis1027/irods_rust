use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

use crate::{
    bosd::{
        xml::{XMLSerializable, XMLSerializableChild},
        Serialiazable,
    },
    error::errors::IrodsError,
    fs::{OpenFlag, OprType},
    tag, tag_fmt,
};

use super::cond_input::CondInput;

#[derive(Debug)]
pub struct CollInp {
    pub name: String,
    flags: i32,
    pub opr_type: OprType,
    pub cond_input: CondInput,
}

impl CollInp {
    pub fn builder() -> CollInpBuilder {
        CollInpBuilder {
            name: String::new(),
            flags: 0,
            opr_type: OprType::No,
            cond_input: CondInput::new(),
        }
    }
}

pub struct CollInpBuilder {
    name: String,
    flags: i32,
    opr_type: OprType,
    cond_input: CondInput,
}

impl CollInpBuilder {
    pub fn set_flag(mut self, flag: OpenFlag) -> Self {
        self.flags |= flag as i32;
        self
    }

    pub fn unset_flag(mut self, flag: OpenFlag) -> Self {
        self.flags &= !(flag as i32);
        self
    }

    pub fn build(self) -> CollInp {
        CollInp {
            name: self.name,
            flags: self.flags,
            opr_type: self.opr_type,
            cond_input: self.cond_input,
        }
    }
}

impl Serialiazable for CollInp {}
impl XMLSerializable for CollInp {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("CollInpNew_PI")))?;

        tag!(writer, "collName", self.name.as_str());
        tag_fmt!(writer, "flags", "{}", self.flags);
        tag_fmt!(writer, "oprType", "{:?}", self.opr_type);

        self.cond_input.to_nested_xml(&mut writer)?;

        writer.write_event(Event::End(BytesEnd::new("CollInpNew_PI")))?;

        Ok(cursor.position() as usize)
    }
}
