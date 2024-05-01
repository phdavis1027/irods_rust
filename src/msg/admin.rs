use derive_builder::Builder;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rand::random;
use std::io::Cursor;

use crate::{
    bosd::{xml::XMLSerializable, Serialiazable},
    tag, AdminOperation, AdminTarget,
};

#[derive(Debug, Builder)]
pub struct GeneralAdminInp {
    action: AdminOperation,
    target: AdminTarget,
    two: String,
    three: String,
    four: String,
    five: String,
    six: String,
    seven: String,
    eight: String,
    nine: String,
}

impl Serialiazable for GeneralAdminInp {}

impl XMLSerializable for GeneralAdminInp {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, crate::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("GeneralAdminInp_PI")))?;

        tag!(writer, "arg0", self.action.into());
        tag!(writer, "arg1", self.target.into());
        tag!(writer, "arg2", self.two.as_str());
        tag!(writer, "arg3", self.three.as_str());
        tag!(writer, "arg4", self.four.as_str());
        tag!(writer, "arg5", self.five.as_str());
        tag!(writer, "arg6", self.six.as_str());
        tag!(writer, "arg7", self.seven.as_str());
        tag!(writer, "arg8", self.eight.as_str());
        tag!(writer, "arg9", self.nine.as_str());

        writer.write_event(Event::End(BytesEnd::new("GeneralAdminInp_PI")))?; // End GeneralAdminInp_PI

        Ok(cursor.position() as usize)
    }
}
