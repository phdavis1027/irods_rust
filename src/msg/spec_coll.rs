use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        xml::{BorrowingXMLSerializable, BorrowingXMLSerializableChild},
        BorrowingSerializable,
    },
    tag, tag_fmt,
};

/*
typedef enum SpecialCollClass {         /* class of SpecColl */
    NO_SPEC_COLL,
    STRUCT_FILE_COLL,
    MOUNTED_COLL,
    LINKED_COLL
} specCollClass_t;
*/

#[cfg_attr(test, derive(Debug))]
pub enum SpecialCollectionClass {
    NoSpecialCollection = 1,
    StructFile = 2,
    Mounted = 3,
    Linked = 4,
}

/*
typedef enum StructFileType {                /* structFile type */
    NONE_STRUCT_FILE_T = 0,   /* no known type */
    HAAW_STRUCT_FILE_T = 1,   /* the UK eScience structFile */
    TAR_STRUCT_FILE_T  = 2,   /* The tar structFile */
    MSSO_STRUCT_FILE_T = 3,   /* The workflow structFile */
} structFileType_t;
*/

#[cfg_attr(test, derive(Debug))]
pub enum StructFileType {
    No = 0,
    Haaw = 1,
    Tar = 2,
    Msso = 3,
}

#[cfg_attr(test, derive(Debug))]
pub struct BorrowingSpecialCollection<'s> {
    class: SpecialCollectionClass,
    struct_file_type: StructFileType,
    collection: &'s str,
    obj_path: &'s str,
    resource: &'s str,
    resc_hier: &'s str,
    phy_path: &'s str,
    cache_dir: &'s str,
    cache_dirty: bool,
    repl_num: i32,
}

impl<'s> BorrowingSerializable<'s> for BorrowingSpecialCollection<'s> {}

impl<'s> BorrowingXMLSerializable<'s> for BorrowingSpecialCollection<'s> {
    fn borrowing_xml_serialize<'r>(
        self,
        sink: &'r mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError>
    where
        's: 'r,
    {
        let mut cursor = Cursor::new(sink);
        let mut writer = quick_xml::Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("SpecColl_PI")))?;

        tag_fmt!(writer, "collClass", "{}", self.class as i32);
        tag_fmt!(writer, "type", "{}", self.struct_file_type as i32);
        tag!(writer, "collection", self.collection);
        tag!(writer, "objPath", self.obj_path);
        tag!(writer, "resource", self.resource);
        tag!(writer, "rescHier", self.resc_hier);
        tag!(writer, "phyPath", self.phy_path);
        tag!(writer, "cacheDir", self.cache_dir);
        tag_fmt!(writer, "cacheDirty", "{}", self.cache_dirty as i32);
        tag_fmt!(writer, "replNum", "{}", self.repl_num);

        writer.write_event(Event::End(BytesEnd::new("SpecColl_PI")))?;

        Ok(cursor.position() as usize)
    }
}

impl<'s> BorrowingXMLSerializableChild<'s> for BorrowingSpecialCollection<'s> {
    fn borrowing_xml_serialize_child<'r, 't1, 't2>(
        self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError>
    where
        's: 'r,
        's: 't1,
        's: 't2,
    {
        writer.write_event(Event::Start(BytesStart::new("SpecColl_PI")))?; // <SpecColl_PI>

        tag_fmt!(writer, "collClass", "{}", self.class as i32);
        tag_fmt!(writer, "type", "{}", self.struct_file_type as i32);
        tag!(writer, "collection", self.collection);
        tag!(writer, "objPath", self.obj_path);
        tag!(writer, "resource", self.resource);
        tag!(writer, "rescHier", self.resc_hier);
        tag!(writer, "phyPath", self.phy_path);
        tag!(writer, "cacheDir", self.cache_dir);
        tag_fmt!(writer, "cacheDirty", "{}", self.cache_dirty as i32);
        tag_fmt!(writer, "replNum", "{}", self.repl_num);

        writer.write_event(Event::End(BytesEnd::new("SpecColl_PI")))?; // </SpecColl_PI>

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn spec_coll_serialize_correctly() {
        let spec_coll = BorrowingSpecialCollection {
            class: SpecialCollectionClass::Linked,
            struct_file_type: StructFileType::Msso,
            collection: "collection",
            obj_path: "obj_path",
            resource: "resource",
            resc_hier: "resc_hier",
            phy_path: "phy_path",
            cache_dir: "cache_dir",
            cache_dirty: true,
            repl_num: 1,
        };

        let mut sink = Vec::new();
        spec_coll.borrowing_xml_serialize(&mut sink).unwrap();

        let expected = r#"<SpecColl_PI><collClass>4</collClass><type>3</type><collection>collection</collection><objPath>obj_path</objPath><resource>resource</resource><rescHier>resc_hier</rescHier><phyPath>phy_path</phy_path><cacheDir>cache_dir</cache_dir><cacheDirty>1</cacheDirty><replNum>1</replNum></SpecColl_PI>"#;
        assert_eq!(String::from_utf8(sink).unwrap(), expected);
    }
}
