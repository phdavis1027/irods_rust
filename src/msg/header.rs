use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Reader, Writer,
};
use rods_prot_msg::error::errors::IrodsError;

use std::io::{self, Cursor, Write};

use crate::{
    bosd::{
        xml::{XMLDeserializable, XMLSerializable},
        Deserializable, Serialiazable,
    },
    tag, tag_fmt,
};

pub const MAX_HEADER_LEN_FOR_XML: usize = 1024;

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
#[repr(u8)]
pub enum MsgType {
    RodsCsNeg,
    RodsApiReq,
    RodsApiReply,
    RodsConnect,
    RodsVersion,
    RodsDisconnect,
}

impl From<&MsgType> for &str {
    fn from(value: &MsgType) -> Self {
        match value {
            MsgType::RodsApiReq => "RODS_API_REQ",
            MsgType::RodsApiReply => "RODS_API_REPLY",
            MsgType::RodsConnect => "RODS_CONNECT",
            MsgType::RodsDisconnect => "RODS_DISCONNECT",
            MsgType::RodsVersion => "RODS_VERSION",
            MsgType::RodsCsNeg => "RODS_CS_NEG_T",
        }
    }
}

impl TryFrom<&str> for MsgType {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "RODS_API_REQ" => MsgType::RodsApiReq,
            "RODS_API_REPLY" => MsgType::RodsApiReply,
            "RODS_CONNECT" => MsgType::RodsConnect,
            "RODS_VERSION" => MsgType::RodsVersion,
            "RODS_CS_NEG_T" => MsgType::RodsCsNeg,
            "RODS_DISCONNECT" => MsgType::RodsDisconnect,
            _ => {
                return Err(IrodsError::Other(format!(
                    "Invalid value for msgType: [{value}]"
                )))
            }
        })
    }
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct StandardHeader {
    pub msg_type: MsgType,
    pub msg_len: usize,
    pub bs_len: usize,
    pub error_len: usize,
    pub int_info: i32,
}

impl StandardHeader {
    pub fn new(
        msg_type: MsgType,
        msg_len: usize,
        bs_len: usize,
        error_len: usize,
        int_info: i32,
    ) -> Self {
        Self {
            msg_type,
            msg_len,
            bs_len,
            error_len,
            int_info,
        }
    }
}

impl Serialiazable for StandardHeader {}
impl XMLSerializable for StandardHeader {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("MsgHeader_PI")))?;

        tag!(writer, "type", (&self.msg_type).into());
        tag_fmt!(writer, "msgLen", "{}", self.msg_len);
        tag_fmt!(writer, "errorLen", "{}", self.error_len);
        tag_fmt!(writer, "bsLen", "{}", self.bs_len);
        tag_fmt!(writer, "intInfo", "{}", self.int_info);

        writer.write_event(Event::End(BytesEnd::new("MsgHeader_PI")))?;
        let len = cursor.position() as usize;

        Ok(len)
    }
}

impl Deserializable for StandardHeader {}
impl XMLDeserializable for StandardHeader {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        unimplemented!()
    }
}
