use std::io::Cursor;

use quick_xml::Writer;
use rods_prot_msg::error::errors::IrodsError;

use crate::common::IrodsProt;

use super::{
    BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializable, BorrowingSerializer,
    IrodsProtocol, OwningDeserializer, OwningSerializer,
};

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

impl IrodsProtocol for XML {
    fn as_enum() -> IrodsProt {
        IrodsProt::XML
    }
}

pub trait BorrowingXMLDeserializable<'r> {
    fn borrowing_xml_deserialize<'s>(src: &'s [u8]) -> Result<Self, IrodsError>
    where
        Self: Sized, // This bound is required to return a Self without dynamic dispatch
        's: 'r; // The deserialization source must live at least
                // as long as the structure referencing it
}

pub trait BorrowingXMLSerializable<'s> {
    fn borrowing_xml_serialize<'r>(self, sink: &'r mut Vec<u8>) -> Result<usize, IrodsError>
    where
        's: 'r;
}

pub trait OwningXMLDeserializable {
    fn owning_xml_deserialize(src: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized;
}

pub trait OwningXMLSerializable {
    fn owning_xml_serialize(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError>;
}

pub trait BorrowingXMLSerializableChild<'s> {
    fn borrowing_xml_serialize_child<'r, 't1, 't2>(
        self,
        writer: &'r mut Writer<&'t1 mut Cursor<&'t2 mut Vec<u8>>>,
    ) -> Result<(), IrodsError>
    where
        's: 'r,
        's: 't1,
        's: 't2;
}

/// Basically all the xml impl of these traits does is
/// define which trait will be called by the structs being
/// serialized

impl BorrowingDeserializer for XML {
    fn rods_borrowing_de<'r, 's, BD>(src: &'s [u8]) -> Result<BD, IrodsError>
    where
        BD: BorrowingDeserializable<'r>,
        's: 'r,
    {
        BD::borrowing_xml_deserialize(src)
    }
}

impl BorrowingSerializer for XML {
    fn rods_borrowing_ser<'r, 's, BS>(src: BS, sink: &'r mut Vec<u8>) -> Result<usize, IrodsError>
    where
        's: 'r,
        BS: BorrowingSerializable<'s>,
    {
        src.borrowing_xml_serialize(sink)
    }
}

impl OwningDeserializer for XML {
    fn rods_owning_de<OD: super::OwningDeserializble>(src: &[u8]) -> Result<OD, IrodsError> {
        OD::owning_xml_deserialize(src)
    }
}

impl OwningSerializer for XML {
    fn rods_owning_ser<OS: super::OwningSerializable>(
        src: &OS,
        sink: &mut Vec<u8>,
    ) -> Result<usize, IrodsError> {
        src.owning_xml_serialize(sink)
    }
}
