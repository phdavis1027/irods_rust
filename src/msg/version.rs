// This struct will probably only be check fleetingly

use std::{borrow::Cow, num::ParseIntError};

use quick_xml::events::Event;
use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::{BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializer};

use crate::bosd::xml::BorrowingXMLDeserializable;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct BorrowingVersion<'s> {
    pub status: i32,
    pub rel_version: (u8, u8, u8),
    pub api_version: Cow<'s, str>,
    pub reconn_port: u32,
    pub reconn_addr: Cow<'s, str>,
    pub cookie: u16,
}

struct RelVersion((u8, u8, u8));
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

impl<'s> BorrowingDeserializable<'s> for BorrowingVersion<'s> {}
impl<'s> BorrowingXMLDeserializable<'s> for BorrowingVersion<'s> {
    fn borrowing_xml_deserialize<'r>(src: &'r [u8]) -> Result<Self, IrodsError>
    where
        'r: 's,
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
        let mut api_version: Option<Cow<'s, str>> = None;
        let mut reconn_port: Option<u32> = None;
        let mut reconn_addr: Option<Cow<'s, str>> = None;
        let mut cookie: Option<u16> = None;

        let mut reader = quick_xml::reader::Reader::from_reader(src);

        let mut state = State::Tag;
        // Basically, this is safe because encountering any invalid input will throw the state
        // machine into a death spiral of reading but not parsing input
        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"Version_PI" => {
                    State::Status
                }
                (State::Tag, Event::Start(e)) => {
                    return Err(
                        rods_prot_msg::error::errors::IrodsError::UnexpectedResponse(
                            // FIXME: This is excessive
                            std::str::from_utf8(e.name().as_ref()).unwrap().into(),
                        ),
                    );
                }

                (State::Status, Event::Start(e)) if e.name().as_ref() == b"status" => {
                    State::StatusInner
                }
                (State::StatusInner, Event::Text(text)) => {
                    status = Some(text.unescape()?.parse()?);

                    State::RelVersion
                }

                (State::RelVersion, Event::Start(e)) if e.name().as_ref() == b"relVersion" => {
                    State::RelVersionInner
                }
                (State::RelVersionInner, Event::Text(text)) => {
                    let v: RelVersion = text.unescape()?.as_ref().try_into()?;
                    rel_version = Some(v.0);

                    State::ApiVersion
                }

                (State::ApiVersion, Event::Start(e)) if e.name().as_ref() == b"apiVersion" => {
                    State::ApiVersionInner
                }
                (State::ApiVersionInner, Event::Text(text)) => {
                    api_version = Some(text.unescape()?);

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
                    reconn_addr = Some(text.unescape()?);

                    State::Cookie
                }

                (State::Cookie, Event::Start(e)) if e.name().as_ref() == b"cookie" => {
                    State::CookieInner
                }
                (State::CookieInner, Event::Text(text)) => {
                    cookie = Some(text.unescape()?.parse()?);

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
                (state, Event::Eof) => {
                    return Err(quick_xml::Error::UnexpectedEof(format!("{state:?}")).into());
                }
                state => state.0, // Hurtle the state machine toward either its inevitable demise or the next data
                                  // value
            };
        }
        // UNSAFE: If any field had been uninitialized, we would have returned an error
        // before this point. State machines!
    }
}

#[cfg(test)]
mod test {
    use crate::bosd::xml::XML;

    use super::*;

    #[test]
    fn borrowed_version_deserialize_correctly() {
        let mut src = r##"
            <Version_PI>
                <status>0</status>
                <relVersion>rods4.3.0</relVersion>
                <apiVersion>d</apiVersion>
                <reconnPort>1247</reconnPort>
                <reconnAddr>0.0.0.0</reconnAddr>
                <cookie>400</cookie>
            </Version_PI>
            "##
        .to_string();

        src.retain(|c| !c.is_whitespace());

        let deserializer = XML;

        let api_version = "d";
        let reconn_addr = "0.0.0.0";

        let expected = BorrowingVersion {
            status: 0,
            rel_version: (4, 3, 0),
            api_version: Cow::from(api_version),
            reconn_port: 1247,
            reconn_addr: Cow::from(reconn_addr),
            cookie: 400,
        };

        assert_eq!(
            expected,
            XML::rods_borrowing_de::<BorrowingVersion>(src.as_bytes()).unwrap()
        );
    }
}
