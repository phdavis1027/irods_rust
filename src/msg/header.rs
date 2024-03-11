use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Reader, Writer,
};
use rods_prot_msg::error::errors::IrodsError;

use std::io::{self, Cursor, Write};

use super::{OwningDe, OwningSer};

pub const MAX_HEADER_LEN_FOR_XML: usize = 1024;

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub enum MsgType {
    RodsCsNeg,
    RodsApiReq,
    RodsApiReply,
    RodsConnect,
    RodsVersion,
    RodsDisconnect,
}

impl From<MsgType> for &str {
    fn from(value: MsgType) -> Self {
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
pub struct OwningStandardHeader {
    pub msg_type: MsgType,
    pub msg_len: usize,
    pub bs_len: usize,
    pub error_len: usize,
    pub int_info: i32,
}

impl OwningStandardHeader {
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

impl OwningSer for OwningStandardHeader {
    fn rods_owning_ser(
        self,
        sink: &mut [u8],
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("MsgHeader_PI")))?;

        writer.write_event(Event::Start(BytesStart::new("type")))?;
        writer.write_event(Event::Text(BytesText::new(self.msg_type.into())))?;
        writer.write_event(Event::End(BytesEnd::new("type")))?;

        writer.write_event(Event::Start(BytesStart::new("msgLen")))?;
        write!(writer.get_mut(), "{}", self.msg_len)?;
        writer.write_event(Event::End(BytesEnd::new("msgLen")))?;

        writer.write_event(Event::Start(BytesStart::new("bsLen")))?;
        write!(writer.get_mut(), "{}", self.bs_len)?;
        writer.write_event(Event::End(BytesEnd::new("bsLen")))?;

        writer.write_event(Event::Start(BytesStart::new("errorLen")))?;
        write!(writer.get_mut(), "{}", self.error_len)?;
        writer.write_event(Event::End(BytesEnd::new("errorLen")))?;

        writer.write_event(Event::Start(BytesStart::new("intInfo")))?;
        write!(writer.get_mut(), "{}", self.error_len)?;
        writer.write_event(Event::End(BytesEnd::new("intInfo")))?;

        writer.write_event(Event::End(BytesEnd::new("MsgHeader_PI")));

        Ok(cursor.position() as usize)
    }
}

impl OwningDe for OwningStandardHeader {
    fn rods_owning_de(
        src: &[u8],
    ) -> Result<super::OwningMsg, rods_prot_msg::error::errors::IrodsError> {
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

        let mut reader = Reader::from_reader(src);

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"MsgHeader_PI" => { State::MsgType
                }
                (State::Tag, Event::Start(e)) => {
                    return Err(IrodsError::UnexpectedResponse(format!("{:?}", e.name().into_inner())))
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

                    State::BsLen
                }

                (State::BsLen, Event::Start(e)) if e.name().as_ref() == b"bsLen" => {
                    State::BsLenInner
                }
                (State::BsLenInner, Event::Text(text)) => {
                    bs_len = Some(text.unescape()?.parse()?);

                    State::ErrorLen
                }

                (State::ErrorLen, Event::Start(e)) if e.name().as_ref() == b"errorLen" => {
                    State::ErrorLenInner
                }
                (State::ErrorLenInner, Event::Text(text)) => {
                    error_len = Some(text.unescape()?.parse()?);

                    State::IntInfo
                }

                (State::IntInfo, Event::Start(e)) if e.name().as_ref() == b"intInfo" => {
                    State::IntInfoInner
                }
                (State::IntInfoInner, Event::Text(text)) => {
                    int_info = Some(text.unescape()?.parse()?);

                    return Ok(crate::msg::OwningMsg::StandardHeader(
                        OwningStandardHeader {
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
                        },
                    ));
                }

                (state, Event::Eof) => {
                    return Err(rods_prot_msg::error::errors::IrodsError::Other(format!(
                        "{state:?}"
                    )))
                }
                state => state.0,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::msg::OwningMsg;

    use super::*;

    #[test]
    fn owning_header_serialize_correctly() {
        let header = OwningStandardHeader::new(MsgType::RodsConnect, 10, 0, 0, 0);

        let mut expected = r##"
            <MsgHeader_PI>
                <type>RODS_CONNECT</type>
                <msgLen>10</msgLen>
                <bsLen>0</bsLen>
                <errorLen>0</errorLen>
                <intInfo>0</intInfo>
            </MsgHeader_PI>
        "##
        .to_string();
        expected.retain(|c| !c.is_whitespace());

        let mut buffer = [0; 1024];
        let bytes_written = header.rods_owning_ser(&mut buffer).unwrap();

        let result = std::str::from_utf8(&buffer[..bytes_written]).unwrap();

        assert_eq!(bytes_written, expected.as_bytes().len());
        assert_eq!(result, expected.as_str());
    }

    #[test]
    fn owning_header_deserialize_correctly() {
        let mut src = r##"
            <MsgHeader_PI>
                <type>RODS_CONNECT</type>
                <msgLen>10</msgLen>
                <bsLen>0</bsLen>
                <errorLen>0</errorLen>
                <intInfo>0</intInfo>
            </MsgHeader_PI>
        "##
        .to_string();
        src.retain(|c| !c.is_whitespace());

        let expected =
            OwningMsg::StandardHeader(OwningStandardHeader::new(MsgType::RodsConnect, 10, 0, 0, 0));

        assert_eq!(
            expected,
            OwningStandardHeader::rods_owning_de(src.as_bytes()).unwrap()
        )
    }
}
