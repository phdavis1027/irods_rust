use std::io::{Cursor, Write};

use crate::{
    bosd::{
        xml::{irods_escapes, XMLSerializable},
        Serialiazable,
    },
    common::IrodsProt,
    error::errors::IrodsError,
    tag, tag_fmt,
};

use quick_xml::{
    escape::escape_with,
    events::{BytesEnd, BytesStart, BytesText, Event},
    Writer,
};

#[derive(Debug)]
pub struct StartupPack {
    pub irods_prot: IrodsProt,
    pub reconn_flag: u32,
    pub connect_cnt: u32,
    pub proxy_user: String,
    pub proxy_zone: String,
    pub client_user: String,
    pub client_zone: String,
    pub rel_version: (u8, u8, u8),
    pub option: String,
}

impl StartupPack {
    pub fn new(
        irods_prot: IrodsProt,
        reconn_flag: u32,
        connect_cnt: u32,
        proxy_user: String,
        proxy_zone: String,
        client_user: String,
        client_zone: String,
        rel_version: (u8, u8, u8),
        option: String,
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
            option,
        }
    }
}

impl Serialiazable for StartupPack {}
impl XMLSerializable for StartupPack {
    fn to_xml(&self, src: &mut Vec<u8>) -> Result<usize, IrodsError> {
        let mut cursor = Cursor::new(src);
        let mut writer = Writer::new(&mut cursor);

        let irods_prot: &str = (&self.irods_prot).into();

        writer.write_event(Event::Start(BytesStart::new("StartupPack_PI")))?;

        tag!(writer, "irodsProt", irods_prot);
        tag_fmt!(writer, "reconnFlag", "{}", self.reconn_flag);
        tag_fmt!(writer, "connectCnt", "{}", self.connect_cnt);
        tag!(
            writer,
            "proxyUser",
            escape_with(&self.proxy_user, irods_escapes).as_ref()
        );
        tag!(
            writer,
            "proxyRcatZone",
            escape_with(&self.proxy_zone, irods_escapes).as_ref()
        );
        tag!(
            writer,
            "clientUser",
            escape_with(&self.client_user, irods_escapes).as_ref()
        );
        tag!(
            writer,
            "clientRcatZone",
            escape_with(&self.client_zone, irods_escapes).as_ref()
        );
        tag_fmt!(
            writer,
            "relVersion",
            "rods{}.{}.{}",
            self.rel_version.0,
            self.rel_version.1,
            self.rel_version.2
        );
        tag!(writer, "apiVersion", "d");
        tag!(
            writer,
            "option",
            escape_with(&self.option, irods_escapes).as_ref()
        );

        writer.write_event(Event::End(BytesEnd::new("StartupPack_PI")))?;

        Ok(cursor.position() as usize)
    }
}
