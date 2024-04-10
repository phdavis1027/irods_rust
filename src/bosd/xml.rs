use std::io::Cursor;

use quick_xml::Writer;
use rods_prot_msg::error::errors::IrodsError;

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

pub(crate) trait XMLDeserializable {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized;
}

pub(crate) trait XMLSerializable {
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError>;
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
