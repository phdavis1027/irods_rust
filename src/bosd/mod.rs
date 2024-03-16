pub mod xml;

use std::io::Cursor;

use rods_prot_msg::error::errors::IrodsError;

use self::xml::{
    BorrowingXMLDeserializable, BorrowingXMLSerializable, OwningXMLDeserializable,
    OwningXMLSerializable,
};

pub trait BorrowingSerializer {
    fn rods_borrowing_ser<'r, 's, BS>(src: &'s BS, sink: &'r mut Vec<u8>) -> Result<usize, IrodsError>
    where
        BS: BorrowingSerializable<'s>,
        's: 'r;
}

pub trait BorrowingDeserializer {
    fn rods_borrowing_de<'r, 's, BD>(src: &'s [u8]) -> Result<BD, IrodsError>
    where
        BD: BorrowingDeserializable<'r>,
        's: 'r;
}

pub trait OwningSerializer {
    fn rods_owning_ser<OS: OwningSerializable>(
        src: &OS,
        sink: &mut Vec<u8>,
    ) -> Result<usize, IrodsError>;
}

pub trait OwningDeserializer {
    fn rods_owning_de<OD: OwningDeserializble>(src: &[u8]) -> Result<OD, IrodsError>;
}

/// To implement a new encoding scheme, you must implemnent four traits corresponding to
/// (Borrowing|Owning)(Serializable|Deserializable) and then add them as beounds
/// the appropriate traits below.

pub trait BorrowingDeserializable<'s>: BorrowingXMLDeserializable<'s> {}
pub trait BorrowingSerializable<'s>: BorrowingXMLSerializable<'s> {}

pub trait OwningSerializable: OwningXMLSerializable {}
pub trait OwningDeserializble: OwningXMLDeserializable {}
