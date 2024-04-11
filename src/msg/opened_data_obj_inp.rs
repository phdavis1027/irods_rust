use std::io::{Cursor, Write};

use quick_xml::events::{BytesEnd, BytesStart, Event};

use crate::{
    bosd::{xml::XMLSerializable, Serialiazable},
    fs::{OprType, Whence},
    tag_fmt,
};

#[derive(Debug)]
pub struct OpenedDataObjInp {
    pub fd: i32,
    pub len: usize,
    pub whence: Whence,
    pub opr_type: OprType,
    pub offset: usize,
    pub bytes_written: usize,
}

impl OpenedDataObjInp {
    pub fn new(
        fd: i32,
        len: usize,
        whence: Whence,
        opr_type: OprType,
        offset: usize,
        bytes_written: usize,
    ) -> Self {
        Self {
            fd,
            len,
            whence,
            opr_type,
            offset,
            bytes_written,
        }
    }
}

impl Serialiazable for OpenedDataObjInp {}
impl XMLSerializable for OpenedDataObjInp {
    fn to_xml(
        &self,
        sink: &mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("OpenDataObjInp_PI")))?;

        tag_fmt!(writer, "l1descInx", "{}", self.fd);
        tag_fmt!(writer, "len", "{}", self.len);
        tag_fmt!(writer, "whence", "{}", self.whence as i32);
        tag_fmt!(writer, "oprType", "{}", self.opr_type as i32);
        tag_fmt!(writer, "offset", "{}", self.offset);
        tag_fmt!(writer, "bytesWritten", "{}", self.bytes_written);

        writer.write_event(Event::End(BytesEnd::new("OpenDataObjInp_PI")))?;

        Ok(cursor.position() as usize)
    }
}
