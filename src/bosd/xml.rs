use std::io::{Cursor, Read};

use crate::error::errors::IrodsError;
use irods_xml::{Reader, Writer};

use crate::common::IrodsProt;

use super::{ProtocolEncoding, Serialiazable};

#[macro_export]
macro_rules! tag {
    ($writer:ident, $name:expr, $value:expr) => {
        $writer.write_event(Event::Start(BytesStart::new($name)))?;
        $writer.write_event(Event::Text(BytesText::new($value)))?;
        $writer.write_event(Event::End(BytesEnd::new($name)))?;
    };
}

#[macro_export]
macro_rules! tag_fmt {
    ($writer:ident, $name:expr, $fmt_str:expr, $($value:expr),*) => {
        $writer.write_event(Event::Start(BytesStart::new($name)))?;
        write!($writer.get_mut(), $fmt_str, $($value),*)?;
        $writer.write_event(Event::End(BytesEnd::new($name)))?;
    };
}

pub struct XML;

pub(crate) fn irods_unescapes<'entity>(s: &str) -> Option<&'entity str> {
    match s {
        "apos" => Some("'"),
        "lt" => Some("<"),
        "gt" => Some(">"),
        "quot" => Some("\""),
        "amp" => Some("&"),
        _ => None,
    }
}

pub(crate) fn irods_escapes<'entity>(c: u8) -> Option<&'entity [u8]> {
    match c {
        b'<' => Some(b"&lt;"),
        b'>' => Some(b"&gt;"),
        b'\'' => Some(b"&apos;"),
        b'"' => Some(b"&quot;"),
        b'`' => Some(b"&apos;"),
        b'&' => Some(b"&amp;"),
        _ => None,
    }
}

pub trait XMLDeserializable {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized;
}

pub trait XMLSerializable {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError>;
}

pub trait XMLSerializableChild {
    fn to_nested_xml<'r, 't1, 't2>(
        &self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError>;
}

pub trait XMLDeserializableChild {
    fn from_nested_xml(reader: Reader<&[u8]>) -> Result<Self, IrodsError>
    where
        Self: Sized;
}

impl ProtocolEncoding for XML {
    fn as_enum() -> IrodsProt {
        IrodsProt::XML
    }

    fn encode<M>(msg: &M, sink: &mut Vec<u8>) -> Result<usize, IrodsError>
    where
        M: Serialiazable,
    {
        // Avoid potential namespace collisions
        XMLSerializable::to_xml(msg, sink)
    }

    fn decode<M>(src: &[u8]) -> Result<M, IrodsError>
    where
        M: super::Deserializable,
    {
        // Avoid potential namespace collisions
        XMLDeserializable::from_xml(src)
    }
}
