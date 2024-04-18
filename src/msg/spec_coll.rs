use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};
use crate::error::errors::IrodsError;

use crate::{
    bosd::{xml::XMLSerializableChild, Serialiazable},
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

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone, Copy)]
pub enum StructFileType {
    No = 0,
    Haaw = 1,
    Tar = 2,
    Msso = 3,
}

#[derive(Debug)]
pub struct SpecialCollection {
    class: SpecialCollectionClass,
    struct_file_type: StructFileType,
    collection: String,
    obj_path: String,
    resource: String,
    resc_hier: String,
    phy_path: String,
    cache_dir: String,
    cache_dirty: bool,
    repl_num: i32,
}

impl XMLSerializableChild for SpecialCollection {
    fn to_nested_xml<'r, 't1, 't2>(
        &self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError> {
        writer.write_event(Event::Start(BytesStart::new("SpecColl_PI")))?; // <SpecColl_PI>

        tag_fmt!(writer, "collClass", "{}", self.class as i32);
        tag_fmt!(writer, "type", "{}", self.struct_file_type as i32);
        tag!(writer, "collection", &self.collection);
        tag!(writer, "objPath", &self.obj_path);
        tag!(writer, "resource", &self.resource);
        tag!(writer, "rescHier", &self.resc_hier);
        tag!(writer, "phyPath", &self.phy_path);
        tag!(writer, "cacheDir", &self.cache_dir);
        tag_fmt!(writer, "cacheDirty", "{}", self.cache_dirty as i32);
        tag_fmt!(writer, "replNum", "{}", self.repl_num);

        writer.write_event(Event::End(BytesEnd::new("SpecColl_PI")))?; // </SpecColl_PI>

        Ok(())
    }
}
