pub mod xml;

use std::{fmt::Debug, io::Cursor};

use crate::error::errors::IrodsError;

use crate::common::IrodsProt;

use self::xml::{XMLDeserializable, XMLSerializable};

/// Note to developers:
/// Connection buffers are a Vec<u8>. The default implementation of `std::io::Write` for Vec<u8>
/// will only ever append to the Vec. Implementations of rods_*_ser must assure that they
/// write to the beginning of the buffer, i.e., we don't care at all what's in the buffer at
/// the start of the function. This is because the buffer is reused across multiple calls to
/// the serialization functions.

pub trait ProtocolEncoding {
    fn encode<M>(msg: &M, sink: &mut Vec<u8>) -> Result<usize, IrodsError>
    where
        M: Serialiazable;

    fn decode<M>(src: &[u8]) -> Result<M, IrodsError>
    where
        M: Deserializable;

    fn as_enum() -> IrodsProt;
}

pub trait Deserializable: XMLDeserializable + Debug {}
pub trait Serialiazable: XMLSerializable + Debug {}
