use std::io::{self, Cursor, Write, Read};

use crate::common::IrodsProt;

use super::BorrowingSer;
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

impl<'s> BorrowingSer<'s> for BorrowingStartupPack<'s> {
    fn rods_borrowing_ser(self, sink: &mut [u8]) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("StartupPack_PI")))?;

        writer.write_event(Event::Start(BytesStart::new("irodsProt")))?;
        writer.write_event(Event::Text(BytesText::new(self.irods_prot.into())))?;
        writer.write_event(Event::End(BytesEnd::new("irodsProt")))?;

        writer.write_event(Event::Start(BytesStart::new("reconnFlag")))?;
        write!(writer.get_mut(), "{}", self.reconn_flag)?;
        writer.write_event(Event::End(BytesEnd::new("reconnFlag")))?;

        writer.write_event(Event::Start(BytesStart::new("connectCnt")))?;
        write!(writer.get_mut(), "{}", self.connect_cnt)?;
        writer.write_event(Event::End(BytesEnd::new("connectCnt")))?;

        writer.write_event(Event::Start(BytesStart::new("proxyUser")))?;
        writer.write_event(Event::Text(BytesText::new(self.proxy_user)))?;
        writer.write_event(Event::End(BytesEnd::new("proxyUser")))?;

        writer.write_event(Event::Start(BytesStart::new("proxyRcatZone")))?;
        writer.write_event(Event::Text(BytesText::new(self.proxy_zone)))?;
        writer.write_event(Event::End(BytesEnd::new("proxyRcatZone")))?;

        writer.write_event(Event::Start(BytesStart::new("clientUser")))?;
        writer.write_event(Event::Text(BytesText::new(self.client_user)))?;
        writer.write_event(Event::End(BytesEnd::new("clientUser")))?;

        writer.write_event(Event::Start(BytesStart::new("clientRcatZone")))?;
        writer.write_event(Event::Text(BytesText::new(self.client_zone)))?;
        writer.write_event(Event::End(BytesEnd::new("clientRcatZone")))?;

        writer.write_event(Event::Start(BytesStart::new("relVersion")))?;
        write!(
            writer.get_mut(),
            "rods{}.{}.{}",
            self.rel_version.0,
            self.rel_version.1,
            self.rel_version.2
        )?;
        writer.write_event(Event::End(BytesEnd::new("relVersion")))?;

        writer.write_event(Event::Start(BytesStart::new("apiVersion")))?;
        writer.write_event(Event::Text(BytesText::new(self.api_version)))?;
        writer.write_event(Event::End(BytesEnd::new("apiVersion")))?;

        writer.write_event(Event::Start(BytesStart::new("option")))?;
        writer.write_event(Event::Text(BytesText::new(self.option)))?;
        writer.write_event(Event::End(BytesEnd::new("option")))?;

        writer.write_event(Event::End(BytesEnd::new("StartupPack_PI")))?;

        Ok(cursor.position() as usize) 
    }
}

mod test {
    use crate::msg::startup_pack;

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
        let mut buffer = [0; 1024];

        let bytes_written = startup_pack.rods_borrowing_ser(&mut buffer).unwrap();
        let result: &str = std::str::from_utf8(&buffer[..bytes_written]).unwrap();

        println!("TEST BYTES WRITTEN: [{}]", bytes_written);
        assert_eq!(bytes_written, expected.as_bytes().len());
        assert_eq!(result, expected.as_str());
    }
}
