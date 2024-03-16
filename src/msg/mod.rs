pub mod header;
pub mod startup_pack;
pub mod version;
pub mod bin_bytes_buf;

use std::io::{self, Read, Write};

use quick_xml::{events::Event, Writer};
use rods_prot_msg::{error::errors::IrodsError, types::Version};

use crate::bosd::xml::BorrowingXMLSerializable;

use self::{
    header::OwningStandardHeader, startup_pack::BorrowingStartupPack, version::BorrowingVersion,
};

#[cfg(test)]
mod test {
    use std::{io::Cursor, net::TcpStream};

    use crate::{
        bosd::{
            xml::XML, BorrowingDeserializer, BorrowingSerializer, OwningDeserializer,
            OwningSerializer,
        },
        common::IrodsProt,
    };

    use super::{
        header::{MsgType, MAX_HEADER_LEN_FOR_XML},
        *,
    };

    #[test]
    fn proof_of_concept_first_two_steps_of_handshake() {
        let mut msg_buf = Cursor::new(vec![0; 1024]);
        let mut header_buf: Cursor<Vec<u8>> = Cursor::new(vec![0; MAX_HEADER_LEN_FOR_XML]);
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

        let msg_len = XML::rods_borrowing_ser(&startup_pack, msg_buf.get_mut()).unwrap();

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);

        let header_len = XML::rods_owning_ser(&header, header_buf.get_mut()).unwrap();

        socket.write(&(header_len as u32).to_be_bytes()).unwrap();
        socket.write(&mut header_buf.get_mut()[..header_len]).unwrap();
        socket.write(&mut msg_buf.get_mut()[..msg_len]).unwrap();

        let mut header_buf_as_slice = &mut header_buf.get_mut().as_mut_slice()[..4];
        socket.read(header_buf_as_slice).unwrap();
        let header_len = u32::from_be_bytes((header_buf_as_slice).try_into().unwrap());

        let mut header_buf_as_slice =
            &mut header_buf.get_mut().as_mut_slice()[..header_len as usize];
        socket.read(header_buf_as_slice).unwrap();
        let header: OwningStandardHeader = XML::rods_owning_de(header_buf_as_slice).unwrap();

        assert_eq!(MsgType::RodsVersion, header.msg_type);
        assert_eq!(0, header.int_info);
        assert_eq!(0, header.bs_len);
        assert_eq!(0, header.error_len);

        socket.read(msg_buf.get_mut()).unwrap();
        let version: BorrowingVersion =
            XML::rods_borrowing_de(msg_buf.get_mut().as_mut_slice()).unwrap();
    }
}
