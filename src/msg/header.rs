use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Reader, Writer,
};
use rods_prot_msg::error::errors::IrodsError;

use std::io::{Cursor, Write};

use crate::{
    bosd::{
        xml::{XMLDeserializable, XMLSerializable},
        Deserializable, Serialiazable,
    },
    tag, tag_fmt,
};

pub const MAX_HEADER_LEN_FOR_XML: usize = 1088;

#[derive(Debug, Eq, PartialEq)]
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

#[derive(Debug, Eq, PartialEq)]
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
        #[derive(Debug)]
        #[repr(u8)]
        enum State {
            Tag,
            MsgType,
            MsgTypeInner,
            MsgLen,
            MsgLenInner,
            BsLen,
            BsLenInner,
            ErrorLen,
            ErrorLenInner,
            IntInfo,
            IntInfoInner,
        }

        let mut msg_type: Option<MsgType> = None;
        let mut msg_len: Option<usize> = None;
        let mut bs_len: Option<usize> = None;
        let mut error_len: Option<usize> = None;
        let mut int_info: Option<i32> = None;

        let mut state = State::Tag;

        let mut reader = Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"MsgHeader_PI" => {
                    State::MsgType
                }
                (State::Tag, Event::Start(e)) => {
                    return Err(IrodsError::UnexpectedResponse(format!(
                        "{:?}",
                        e.name().into_inner()
                    )))
                }

                (State::MsgType, Event::Start(e)) if e.name().as_ref() == b"type" => {
                    State::MsgTypeInner
                }
                (State::MsgTypeInner, Event::Text(text)) => {
                    msg_type = Some(text.unescape()?.as_ref().try_into()?);
                    State::MsgLen
                }

                (State::MsgLen, Event::Start(e)) if e.name().as_ref() == b"msgLen" => {
                    State::MsgLenInner
                }
                (State::MsgLenInner, Event::Text(text)) => {
                    msg_len = Some(text.unescape()?.parse()?);

                    State::ErrorLen
                }

                (State::ErrorLen, Event::Start(e)) if e.name().as_ref() == b"errorLen" => {
                    State::ErrorLenInner
                }
                (State::ErrorLenInner, Event::Text(text)) => {
                    error_len = Some(text.unescape()?.parse()?);

                    State::BsLen
                }

                (State::BsLen, Event::Start(e)) if e.name().as_ref() == b"bsLen" => {
                    State::BsLenInner
                }
                (State::BsLenInner, Event::Text(text)) => {
                    bs_len = Some(text.unescape()?.parse()?);

                    State::IntInfo
                }

                (State::IntInfo, Event::Start(e)) if e.name().as_ref() == b"intInfo" => {
                    State::IntInfoInner
                }
                (State::IntInfoInner, Event::Text(text)) => {
                    int_info = Some(text.unescape()?.parse()?);

                    return Ok(StandardHeader {
                        msg_type: msg_type.ok_or(IrodsError::Other(
                            "Failed to parse field msgType of header".into(),
                        ))?,
                        msg_len: msg_len.ok_or(IrodsError::Other(
                            "Failed to parse field msgLen of header".into(),
                        ))?,
                        bs_len: bs_len.ok_or(IrodsError::Other(
                            "Failed to parse field bsLen of header".into(),
                        ))?,
                        error_len: error_len.ok_or(IrodsError::Other(
                            "Failed to parse field errorLen of header".into(),
                        ))?,
                        int_info: int_info.ok_or(IrodsError::Other(
                            "Failed to parse field intInfo of header".into(),
                        ))?,
                    });
                }

                (state, Event::Eof) => {
                    return Err(rods_prot_msg::error::errors::IrodsError::Other(format!(
                        "{state:?}"
                    )));
                }
                state => state.0,
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct HandshakeHeader {
    algo: String,
    key_size: usize,
    salt_size: usize,
    hash_rounds: usize,
}

impl HandshakeHeader {
    pub fn new(algo: String, key_size: usize, salt_size: usize, hash_rounds: usize) -> Self {
        Self {
            algo,
            key_size,
            salt_size,
            hash_rounds,
        }
    }
}

impl Serialiazable for HandshakeHeader {}
impl XMLSerializable for HandshakeHeader {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("MsgHeader_PI")))?;

        tag!(writer, "type", &self.algo);
        tag_fmt!(writer, "msgLen", "{}", self.key_size);
        tag_fmt!(writer, "errorLen", "{}", self.salt_size);
        tag_fmt!(writer, "bsLen", "{}", self.hash_rounds);
        tag!(writer, "intInfo", "0");

        writer.write_event(Event::End(BytesEnd::new("MsgHeader_PI")))?;

        Ok(cursor.position() as usize)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SharedSecretHeader {
    pub size: usize,
}

impl Serialiazable for SharedSecretHeader {}
impl XMLSerializable for SharedSecretHeader {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("MsgHeader_PI")))?;

        tag!(writer, "type", "SHARED_SECRET");
        tag_fmt!(writer, "msgLen", "{}", self.size);
        tag!(writer, "errorLen", "0");
        tag!(writer, "bsLen", "0");
        tag!(writer, "intInfo", "0");

        writer.write_event(Event::End(BytesEnd::new("MsgHeader_PI")))?;

        Ok(cursor.position() as usize)
    }
}
