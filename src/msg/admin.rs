use derive_builder::Builder;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use std::io::Cursor;

use crate::{
    bosd::{xml::XMLSerializable, Serialiazable},
    tag,
};

#[derive(Debug, Default, Builder)]
pub struct GeneralAdminInp {
    pub zero: String,
    pub one: String,
    pub two: String,
    pub three: String,
    pub four: String,
    pub five: String,
    pub six: String,
    pub seven: String,
    pub eight: String,
    pub nine: String,
}

impl Serialiazable for GeneralAdminInp {}

impl XMLSerializable for GeneralAdminInp {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, crate::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("GeneralAdminInp_PI")))?;

        tag!(writer, "arg0", self.zero.as_str());
        tag!(writer, "arg1", self.one.as_str());
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
