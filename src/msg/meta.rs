use std::io::Cursor;

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

use crate::{
    bosd::{xml::XMLSerializable, Serialiazable},
    error::errors::IrodsError,
    tag, AVUOperation, AVUTarget, AVU,
};

#[derive(Debug)]
pub struct ModAVURequest {
    op: AVUOperation,
    target_type: AVUTarget,
    target_name: String,
    avu: AVU,
    new_avu: Option<AVU>,
}

impl ModAVURequest {
    pub fn new(
        op: AVUOperation,
        target_type: AVUTarget,
        target_name: String,
        avu: AVU,
        new_avu: Option<AVU>,
    ) -> Self {
        Self {
            op,
            target_type,
            target_name,
            avu,
            new_avu,
        }
    }
}

impl Serialiazable for ModAVURequest {}

impl XMLSerializable for ModAVURequest {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("ModAVUMetaDataInp_PI")))?;

        tag!(writer, "arg0", self.op.into());
        tag!(writer, "arg1", self.target_type.into());
        tag!(writer, "arg2", self.target_name.as_str());
        tag!(writer, "arg3", self.avu.attribute.as_str());
        tag!(writer, "arg4", self.avu.value.as_str());
        tag!(writer, "arg5", self.avu.unit.as_str());
        match &self.new_avu {
            Some(new_avu) => {
                tag!(writer, "arg6", new_avu.attribute.as_str());
                tag!(writer, "arg7", new_avu.value.as_str());
                tag!(writer, "arg8", new_avu.unit.as_str());
            }
            None => {
                tag!(writer, "arg6", "");
                tag!(writer, "arg7", "");
                tag!(writer, "arg8", "");
            }
        };
        tag!(writer, "arg9", "");

        writer.write_event(Event::End(BytesEnd::new("ModAVUMetaDataInp_PI")))?;

        Ok(cursor.position() as usize)
    }
}
