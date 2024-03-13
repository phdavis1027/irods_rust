use rods_prot_msg::error::errors::IrodsError;

use super::{
    BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializable, BorrowingSerializer,
    OwningDeserializer, OwningSerializer,
};

// Empty struct to act as a generic parameters and hold the relevant traits

pub struct XML;

pub trait BorrowingXMLDeserializable<'s> {
    fn borrowing_xml_deserialize<'r>(src: &'r [u8]) -> Result<Self, IrodsError>
    where
        Self: Sized, // This bound is required to return a Self without dynamic dispatch
        'r: 's; // The deserialization source must live at least
                // as long as the structure referencing it
}

pub trait BorrowingXMLSerializable<'s> {
    fn borrowing_xml_serialize<'r> (&'s self, sink: &'r mut [u8]) -> Result<usize, IrodsError>
    where
        Self: Sized, 
        's: 'r;
}

pub trait OwningXMLDeserializable {
    fn owning_xml_deserialize(src: &[u8]) -> Result<Self, IrodsError> where Self: Sized;
}

pub trait OwningXMLSerializable {
    fn owning_xml_serialize(&self, sink: &mut [u8]) -> Result<usize, IrodsError>;
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
    fn rods_borrowing_ser<'r, 's, BS>(
        src: &'s BS,
        sink: &'r mut [u8],
    ) -> Result<usize, IrodsError> 
        where 's: 'r,
        BS: BorrowingSerializable<'s>
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
        sink: &mut [u8],
    ) -> Result<usize, IrodsError> {
        src.owning_xml_serialize(sink)
    }
}
