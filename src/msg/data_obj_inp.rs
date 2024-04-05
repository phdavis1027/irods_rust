use std::io::{Cursor, Write};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};

use crate::{
    bosd::{
        xml::{BorrowingXMLSerializable, BorrowingXMLSerializableChild},
        BorrowingSerializable,
    },
    fs::{CreateMode, OpenFlags, OprType},
    tag, tag_fmt,
};

use super::{cond_input::BorrowingCondInput, spec_coll::BorrowingSpecialCollection};

#[cfg_attr(test, derive(Debug))]
pub struct BorrowingDataObjInp<'s> {
    pub path: &'s str,
    create_mode: i32, // Use create
    open_flags: i32,
    pub opr_type: OprType,
    pub offset: i64,
    pub data_size: i32,
    pub num_threads: i32,
    pub spec_coll: Option<BorrowingSpecialCollection<'s>>,
    pub cond_input: BorrowingCondInput<'s>,
}

impl<'s> BorrowingDataObjInp<'s> {
    pub(crate) fn new(path: &'s str, opr_type: OprType, open_flags: i32) -> Self {
        Self {
            path,
            opr_type,
            create_mode: 0,
            open_flags,
            offset: 0,
            data_size: 0,
            num_threads: 0,
            spec_coll: None,
            cond_input: BorrowingCondInput::new(),
        }
    }

    pub fn set_create_mode(mut self, create_mode: CreateMode) {
        self.create_mode |= create_mode as i32;
    }

    pub fn unset_create_mode(mut self, create_mode: CreateMode) {
        self.create_mode &= !(create_mode as i32);
    }

    pub fn set_open_flags(mut self, open_flags: OpenFlags) {
        self.open_flags |= open_flags as i32;
    }

    pub fn unset_open_flags(mut self, open_flags: OpenFlags) {
        self.open_flags &= !(open_flags as i32);
    }
}

impl<'s> BorrowingSerializable<'s> for BorrowingDataObjInp<'s> {}
impl<'s> BorrowingXMLSerializable<'s> for BorrowingDataObjInp<'s> {
    fn borrowing_xml_serialize<'r>(
        self,
        sink: &'r mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError>
    where
        's: 'r,
    {
        let mut cursor = Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("DataObjInp_PI")))?;

        tag!(writer, "objPath", self.path);
        tag_fmt!(writer, "createMode", "{}", self.create_mode);
        tag_fmt!(writer, "openFlags", "{}", self.open_flags);
        tag_fmt!(writer, "oprType", "{}", self.opr_type as i32);
        tag_fmt!(writer, "offset", "{}", self.offset);
        tag_fmt!(writer, "dataSize", "{}", self.data_size);
        tag_fmt!(writer, "numThreads", "{}", self.num_threads);

        if let Some(spec_coll) = self.spec_coll {
            spec_coll.borrowing_xml_serialize_child(&mut writer)?;
        }

        self.cond_input.borrowing_xml_serialize_child(&mut writer)?;

        writer.write_event(Event::End(BytesEnd::new("DataObjInp_PI")))?;

        Ok(cursor.position() as usize)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_borrowing_data_obj_inp() {
        let data_obj_inp =
            BorrowingDataObjInp::new("path/to/data", OprType::Put, OpenFlags::ReadOnly as i32);

        let mut sink = Vec::new();
        data_obj_inp.borrowing_xml_serialize(&mut sink).unwrap();
        let xml = std::str::from_utf8(&sink).unwrap();

        assert_eq!(
            xml,
            r#"<DataObjInp_PI><objPath>path/to/data</objPath><createMode>0</createMode><openFlags>0</openFlags><oprType>201</oprType><offset>0</offset><dataSize>0</dataSize><numThreads>0</numThreads><SpecColl_PI></SpecColl_PI><KeyValPair_PI><ssLen>0</ssLen></KeyValPair_PI></DataObjInp_PI>"#
        );
    }
}
