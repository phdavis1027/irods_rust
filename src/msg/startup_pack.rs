use std::io::{self, Cursor, Read, Write};

use crate::{tag, bosd::{xml::{BorrowingXMLSerializable}, BorrowingSerializable}, common::IrodsProt, tag_fmt};

use rods_prot_msg::error::errors::IrodsError;

use quick_xml::{
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

#[cfg_attr(feature = "arbitrary", Arbitrary)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct BorrowingStartupPack<'s> {
    pub irods_prot: IrodsProt,
    pub reconn_flag: u32,
    pub connect_cnt: u32,
    pub proxy_user: &'s str,
    pub proxy_zone: &'s str,
    pub client_user: &'s str,
    pub client_zone: &'s str,
    pub rel_version: (u8, u8, u8),
    pub api_version: &'s str,
    pub option: &'s str,
}

impl<'s> BorrowingStartupPack<'s> {
    pub fn new(
        irods_prot: IrodsProt,
        reconn_flag: u32,
        connect_cnt: u32,
        proxy_user: &'s str,
        proxy_zone: &'s str,
        client_user: &'s str,
        client_zone: &'s str,
        rel_version: (u8, u8, u8),
        api_version: &'s str,
        option: &'s str,
    ) -> Self {
        Self {
            irods_prot,
            reconn_flag,
            connect_cnt,
            proxy_user,
            proxy_zone,
            client_user,
            client_zone,
            rel_version,
            api_version,
            option,
        }
    }
}

impl<'s> BorrowingSerializable<'s> for BorrowingStartupPack<'s> {}
impl<'s> BorrowingXMLSerializable<'s> for BorrowingStartupPack<'s> {
    fn borrowing_xml_serialize<'r>(&self, sink: &'r mut Vec<u8>) -> Result<usize, IrodsError>
    where
        's: 'r,
    {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("StartupPack_PI")))?;
        tag!(writer, "irodsProt", (&self.irods_prot).into());
        tag_fmt!(writer, "reconnFlag", "{}", self.reconn_flag);
        tag_fmt!(writer, "connectCnt", "{}", self.connect_cnt);
        tag!(writer, "proxyUser", self.proxy_user);
        tag!(writer, "proxyRcatZone", self.proxy_zone);
        tag!(writer, "clientUser", self.client_user);
        tag!(writer, "clientRcatZone", self.client_zone);
        tag_fmt!(
            writer,
            "relVersion",
            "rods{}.{}.{}",
            self.rel_version.0,
            self.rel_version.1,
            self.rel_version.2
        );
        tag!(writer, "apiVersion", self.api_version);
        tag!(writer, "option", self.option);
        writer.write_event(Event::End(BytesEnd::new("StartupPack_PI")))?;

        Ok(cursor.position() as usize)
    }
}

mod test {
    use crate::{
        bosd::{xml::XML, BorrowingSerializer, BorrowingSerializable},
        msg::startup_pack,
    };

    use super::*;

    #[test]
    fn borrowing_startup_pack_correct_serialization() {
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
        // NOTE: In real code, use a BufWriter

        let mut expected = String::from(
            r##"
            <StartupPack_PI>
                <irodsProt>1</irodsProt>
                <reconnFlag>0</reconnFlag>
                <connectCnt>0</connectCnt>
                <proxyUser>rods</proxyUser>
                <proxyRcatZone>tempZone</proxyRcatZone>
                <clientUser>rods</clientUser>
                <clientRcatZone>tempZone</clientRcatZone>
                <relVersion>rods4.3.0</relVersion>
                <apiVersion>d</apiVersion>
                <option>packe</option>
            </StartupPack_PI>
            "##,
        );
        expected.retain(|c| !c.is_whitespace());

        let mut buffer = Vec::new();

        let bytes_written = XML::rods_borrowing_ser(&startup_pack, &mut buffer)
            .unwrap();
        let result: &str = std::str::from_utf8(&buffer[..bytes_written]).unwrap();

        assert_eq!(result, expected.as_str());
    }
}