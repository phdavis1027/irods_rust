use std::{
    io::{Cursor, Write},
    path::PathBuf,
    str::FromStr,
};

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Reader, Writer,
};

use crate::{
    bosd::{
        xml::{irods_unescapes, XMLDeserializable, XMLSerializable, XMLSerializableChild},
        Deserializable, Serialiazable,
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
        tag_fmt!(writer, "oprType", "{:?}", self.opr_type as i32);

        self.cond_input.to_nested_xml(&mut writer)?;

        writer.write_event(Event::End(BytesEnd::new("CollInpNew_PI")))?;

        Ok(cursor.position() as usize)
    }
}

/*
<CollOprStat_PI>
<filesCnt>0</filesCnt>
<totalFileCnt>0</totalFileCnt>
<bytesWritten>0</bytesWritten>
<lastObjPath></lastObjPath>
</CollOprStat_PI>
*/

#[derive(Debug)]
pub struct CollOprStat {
    pub files_cnt: i32,
    pub total_file_cnt: i32,
    pub bytes_written: i64,
    pub last_obj_path: PathBuf,
}

impl Deserializable for CollOprStat {}
impl XMLDeserializable for CollOprStat {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            FilesCnt,
            FilesCntInner,
            TotalFileCnt,
            TotalFileCntInner,
            BytesWritten,
            BytesWrittenInner,
            LastObjPath,
            LastObjPathInner,
        }

        let mut state = State::Tag;
        let mut files_cnt: Option<i32> = None;
        let mut total_file_cnt: Option<i32> = None;
        let mut bytes_written: Option<i64> = None;
        let mut last_obj_path: Option<PathBuf> = None;

        let mut reader = Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"CollOprStat_PI" => {
                    State::FilesCnt
                }
                (State::FilesCnt, Event::Start(e)) if e.name().as_ref() == b"filesCnt" => {
                    State::FilesCntInner
                }
                (State::FilesCntInner, Event::Text(e)) => {
                    files_cnt = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::TotalFileCnt
                }
                (State::TotalFileCnt, Event::Start(e)) if e.name().as_ref() == b"totalFileCnt" => {
                    State::TotalFileCntInner
                }
                (State::TotalFileCntInner, Event::Text(e)) => {
                    total_file_cnt = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::BytesWritten
                }
                (State::BytesWritten, Event::Start(e)) if e.name().as_ref() == b"bytesWritten" => {
                    State::BytesWrittenInner
                }
                (State::BytesWrittenInner, Event::Text(e)) => {
                    bytes_written = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::LastObjPath
                }
                (State::LastObjPath, Event::Start(e)) if e.name().as_ref() == b"lastObjPath" => {
                    State::LastObjPathInner
                }
                (State::LastObjPathInner, Event::Text(e)) => {
                    last_obj_path = Some(
                        PathBuf::from_str(e.unescape_with(irods_unescapes)?.as_ref())
                            .map_err(|_| IrodsError::Other("Invalid path".to_string()))?,
                    );

                    return Ok(CollOprStat {
                        files_cnt: files_cnt
                            .ok_or_else(|| IrodsError::Other("Missing filesCnt".to_string()))?,
                        total_file_cnt: total_file_cnt
                            .ok_or_else(|| IrodsError::Other("Missing totalFileCnt".to_string()))?,
                        bytes_written: bytes_written
                            .ok_or_else(|| IrodsError::Other("Missing bytesWritten".to_string()))?,
                        last_obj_path: last_obj_path
                            .ok_or_else(|| IrodsError::Other("Missing lastObjPath".to_string()))?,
                    });
                }
                (_, Event::Eof) => {
                    return Err(IrodsError::Other("Unexpected EOF".to_string()));
                }
                state => state.0,
            }
        }
    }
}
