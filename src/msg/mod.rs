pub mod header;
pub mod startup_pack;
pub mod version;

use std::io::{self, Write, Read};

use quick_xml::{events::Event, Writer};
use rods_prot_msg::{error::errors::IrodsError, types::Version};

use self::{
    header::OwningStandardHeader, startup_pack::BorrowingStartupPack, version::BorrowingVersion,
};

// This primarily for when you want to
// send a message, i.e., for when the msg
// only needs to exist for long enough to
// throw it over the wire.
#[cfg_attr(feature = "arbitrary", Arbitrary)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum BorrowingMsg<'s> {
    StartupPackPI(BorrowingStartupPack<'s>),
    VersionPI(BorrowingVersion<'s>),
}

#[cfg_attr(feature = "arbitrary", Arbitrary)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum OwningMsg {
    StandardHeader(OwningStandardHeader),
}

pub trait BorrowingSer<'s> {
    fn rods_borrowing_ser(self, sink: &mut [u8]) -> Result<usize, IrodsError>;
}

pub trait BorrowingDe<'s> {
    fn rods_borrowing_de(src: &'s [u8]) -> Result<BorrowingMsg<'s>, IrodsError>;
}

pub trait OwningSer {
    fn rods_owning_ser(self, sink: &mut [u8]) -> Result<usize, IrodsError>;
}

pub trait OwningDe {
    fn rods_owning_de(src: &[u8]) -> Result<OwningMsg, IrodsError>;
}

#[cfg(test)]
mod test {
    use std::net::TcpStream;

    use crate::common::IrodsProt;

    use super::{
        header::{MsgType, MAX_HEADER_LEN_FOR_XML},
        *,
    };

    #[test]
    fn proof_of_concept_first_two_steps_of_handshake() {
        let mut buf = [0; 2048];
        let addr = "172.27.0.3";
        let mut socket = TcpStream::connect((addr, 1247)).unwrap();

        let startup_pack = BorrowingStartupPack::new(
            IrodsProt::XML,
            0,
            0,
            "rods",
            "tempZone",
            "rods",
            "tempZone",
            (4, 3, 0),
            "d",
            "packe",
        );

        let msg_len = startup_pack
            .rods_borrowing_ser(&mut buf[4 + MAX_HEADER_LEN_FOR_XML..])
            .unwrap();
        println!("MSG_LEN: [{}]", msg_len);
        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);

        let header_len = header
            .rods_owning_ser(&mut buf[4..4 + MAX_HEADER_LEN_FOR_XML])
            .unwrap();
        println!("HEADER LEN: [{}]", header_len);

        socket.write(&(header_len as u32).to_be_bytes()).unwrap();
        socket.write(&buf[4..4 + header_len]).unwrap();
        socket
            .write(&buf[4 + MAX_HEADER_LEN_FOR_XML..4 + MAX_HEADER_LEN_FOR_XML + msg_len])
            .unwrap();

        socket.read(&mut buf[..4]).unwrap();
        let header_len = u32::from_be_bytes((&buf[..4]).try_into().unwrap());

        socket.read(&mut buf[..header_len as usize]).unwrap();
        let header = match OwningStandardHeader::rods_owning_de(&buf[4..header_len as usize]).unwrap() {
           OwningMsg::StandardHeader(header) => header 
        };

        assert_eq!(MsgType::RodsVersion, header.msg_type);
        assert_eq!(0, header.int_info);
        assert_eq!(0, header.bs_len);
        assert_eq!(0, header.error_len);

        socket.read(&mut buf[..header.msg_len]).unwrap();
        let version = BorrowingVersion::rods_borrowing_de(&buf[..header.msg_len]).unwrap();
    }
}
