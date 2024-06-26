// This struct will probably only be check fleetingly

use std::num::ParseIntError;

use crate::{bosd::xml::irods_unescapes, error::errors::IrodsError};
use quick_xml::events::Event;

use crate::bosd::{xml::XMLDeserializable, Deserializable};

#[derive(Debug, PartialEq, Eq)]
pub struct Version {
    pub status: i32,
    pub rel_version: (u8, u8, u8),
    pub api_version: String,
    pub reconn_port: u32,
    pub reconn_addr: String,
    pub cookie: u16,
}

pub struct RelVersion((u8, u8, u8));
impl TryFrom<&str> for RelVersion {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() <= 4 {
            return Err(IrodsError::Other(format!("bad relVersion: [{value}]")));
        }

        // Idiomatic!
        Ok(RelVersion(value[4..].splitn(3, ".").enumerate().try_fold(
            (0, 0, 0),
            |mut acc, (n, c)| {
                match n {
                    0 => acc.0 = c.parse::<u8>()?,
                    1 => acc.1 = c.parse::<u8>()?,
                    2 => acc.2 = c.parse::<u8>()?,
                    _ => unreachable!("call to splitn returns at most 3 elements"),
                };
                Ok::<(u8, u8, u8), ParseIntError>(acc)
            },
        )?))
    }
}

impl Deserializable for Version {}
impl XMLDeserializable for Version {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[derive(Debug)]
        #[repr(u8)]
        enum State {
            Tag,
            Status,
            StatusInner,
            RelVersion,
            RelVersionInner,
            ApiVersion,
            ApiVersionInner,
            ReconnPort,
            ReconnPortInner,
            ReconnAddr,
            ReconnAddrInner,
            Cookie,
            CookieInner,
        }

        let mut status: Option<i32> = None;
        let mut rel_version: Option<(u8, u8, u8)> = None;
        let mut api_version: Option<String> = None;
        let mut reconn_port: Option<u32> = None;
        let mut reconn_addr: Option<String> = None;
        let mut cookie: Option<u16> = None;

        let mut reader = quick_xml::reader::Reader::from_reader(xml);

        let mut state = State::Tag;
        // Basically, this is safe because encountering any invalid input will throw the state
        // machine into a death spiral of reading but not parsing input
        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"Version_PI" => {
                    State::Status
                }
                (State::Tag, Event::Start(e)) => {
                    return Err(crate::error::errors::IrodsError::UnexpectedResponse(
                        // FIXME: This is excessive
                        std::str::from_utf8(e.name().as_ref()).unwrap().into(),
                    ));
                }

                (State::Status, Event::Start(e)) if e.name().as_ref() == b"status" => {
                    State::StatusInner
                }
                (State::StatusInner, Event::Text(text)) => {
                    status = Some(text.unescape_with(irods_unescapes)?.parse()?);

                    State::RelVersion
                }

                (State::RelVersion, Event::Start(e)) if e.name().as_ref() == b"relVersion" => {
                    State::RelVersionInner
                }
                (State::RelVersionInner, Event::Text(text)) => {
                    let v: RelVersion = text.unescape_with(irods_unescapes)?.as_ref().try_into()?;
                    rel_version = Some(v.0);

                    State::ApiVersion
                }

                (State::ApiVersion, Event::Start(e)) if e.name().as_ref() == b"apiVersion" => {
                    State::ApiVersionInner
                }
                (State::ApiVersionInner, Event::Text(text)) => {
                    api_version = Some(text.unescape_with(irods_unescapes)?.to_string());

                    State::ReconnPort
                }

                (State::ReconnPort, Event::Start(e)) if e.name().as_ref() == b"reconnPort" => {
                    State::ReconnPortInner
                }
                (State::ReconnPortInner, Event::Text(text)) => {
                    reconn_port = Some(text.unescape()?.parse()?);

                    State::ReconnAddr
                }

                (State::ReconnAddr, Event::Start(e)) if e.name().as_ref() == b"reconnAddr" => {
                    State::ReconnAddrInner
                }
                (State::ReconnAddrInner, Event::Text(text)) => {
                    reconn_addr = Some(text.unescape_with(irods_unescapes)?.to_string());

                    State::Cookie
                }

                (State::Cookie, Event::Start(e)) if e.name().as_ref() == b"cookie" => {
                    State::CookieInner
                }
                (State::CookieInner, Event::Text(text)) => {
                    cookie = Some(text.unescape_with(irods_unescapes)?.parse()?);

                    return Ok(Self {
                        status: status.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field status".into(),
                        ))?,
                        rel_version: rel_version.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field rel_version".into(),
                        ))?,
                        api_version: api_version.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field api_version".into(),
                        ))?,
                        reconn_port: reconn_port.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field reconn_port".into(),
                        ))?,
                        reconn_addr: reconn_addr.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field reconn_addr".into(),
                        ))?,
                        cookie: cookie.ok_or(IrodsError::Other(
                            "Failed to parse Version_PI field cookie".into(),
                        ))?,
                    });
                }
                (_, Event::Eof) => {
                    return Err(IrodsError::Other("Unexpected EOF".into()));
                }
                state => state.0, // Hurtle the state machine toward either its inevitable demise or the next data
                                  // value
            };
        }
        // UNSAFE: If any field had been uninitialized, we would have returned an error
        // before this point. State machines!
    }
}
