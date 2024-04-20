use std::io::{Cursor, Write};

use irods_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::{
    bosd::{
        xml::{XMLSerializable, XMLSerializableChild},
        Serialiazable,
    },
    fs::{CreateMode, OpenFlag, OprType},
    tag, tag_fmt,
};

use super::{cond_input::CondInput, spec_coll::SpecialCollection};

#[derive(Debug)]
pub struct DataObjInp {
    pub path: String,
    create_mode: i32, // Use create
    open_flags: i32,
    pub opr_type: OprType,
    pub offset: i64,
    pub data_size: i32,
    pub num_threads: i32,
    pub spec_coll: Option<SpecialCollection>,
    pub cond_input: CondInput,
}

impl DataObjInp {
    pub(crate) fn new(path: String, opr_type: OprType, open_flags: i32, create_mode: i32) -> Self {
        Self {
            path,
            opr_type,
            create_mode,
            open_flags,
            offset: 0,
            data_size: 0,
            num_threads: 0,
            spec_coll: None,
            cond_input: CondInput::new(),
        }
    }
}

impl Serialiazable for DataObjInp {}
impl XMLSerializable for DataObjInp {
    fn to_xml(
        &self,
        sink: &mut Vec<u8>,
    ) -> Result<usize, crate::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = irods_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("DataObjInp_PI")))?;

        tag!(writer, "objPath", &self.path);
        tag_fmt!(writer, "createMode", "{}", self.create_mode);
        tag_fmt!(writer, "openFlags", "{}", self.open_flags);
        tag_fmt!(writer, "offset", "{}", self.offset);
        tag_fmt!(writer, "dataSize", "{}", self.data_size);
        tag_fmt!(writer, "numThreads", "{}", self.num_threads);
        tag_fmt!(writer, "oprType", "{}", self.opr_type as i32);

        if let Some(ref spec_coll) = self.spec_coll {
            spec_coll.to_nested_xml(&mut writer)?;
        }

        self.cond_input.to_nested_xml(&mut writer)?;

        writer.write_event(Event::End(BytesEnd::new("DataObjInp_PI")))?;

        Ok(cursor.position() as usize)
    }
}
