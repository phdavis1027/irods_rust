use std::{
    io::{Cursor, Write},
    path::PathBuf,
};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

use crate::{
    bosd::{xml::XMLSerializable, Serialiazable},
    common::AccessLevel,
    error::errors::IrodsError,
    tag, tag_fmt,
};

#[derive(Debug)]
pub struct ModifyAccessRequest {
    recursive: bool,
    access_level: AccessLevel,
    user_name: String,
    zone: String,
    path: PathBuf,
}

impl ModifyAccessRequest {
    pub fn new(
        recursive: bool,
        access_level: AccessLevel,
        user_name: String,
        zone: String,
        path: PathBuf,
    ) -> Self {
        Self {
            recursive,
            access_level,
            user_name,
            zone,
            path,
        }
    }
}

impl Serialiazable for ModifyAccessRequest {}
impl XMLSerializable for ModifyAccessRequest {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("modAccessControl_PI")))?;

        tag_fmt!(writer, "recursive", "{}", self.recursive as i32);
        tag!(writer, "accessLevel", self.access_level.into());
        tag!(writer, "userName", self.user_name.as_str());
        tag!(writer, "zone", self.zone.as_str());
        tag!(writer, "path", self.path.to_str().unwrap());

        writer.write_event(Event::End(BytesEnd::new("modAccessControl_PI")))?;

        Ok(cursor.position() as usize)
    }
}
