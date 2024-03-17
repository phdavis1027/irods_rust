use std::{io::{self, Write, Read}, marker::PhantomData, net::TcpStream, time::Duration};

use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        BorrowingDeserializer, BorrowingSerializer, IrodsProtocol, OwningDeserializer,
        OwningSerializer,
    },
    msg::{startup_pack::BorrowingStartupPack, header::{OwningStandardHeader, MsgType}, version::BorrowingVersion},
};

use super::{Account, ConnConfig, Connection, CsNegPolicy};

impl ConnConfig<TcpStream> {
    fn new(
        buf_size: usize,
        request_timeout: Duration,
        read_timeout: Duration,
        host: String,
        port: u16,
        a_ttl: u32,
    ) -> Self {
        Self {
            a_ttl,
            buf_size,
            request_timeout,
            read_timeout,
            addr: (host, port),
            cs_neg_policy: CsNegPolicy::Refuse,
            ssl_config: None,
            phantom_transport: PhantomData,
        }
    }
}

impl Default for ConnConfig<TcpStream> {
    fn default() -> Self {
        Self {
            buf_size: 8092,
            request_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(5),
            // FIXME: Change this
            cs_neg_policy: CsNegPolicy::Refuse,
            ssl_config: None,
            addr: ("172.27.0.3".into(), 1247),
            a_ttl: 30,
            phantom_transport: PhantomData,
        }
    }
}

impl<T> Connection<T, TcpStream>
where
    T: BorrowingSerializer + BorrowingDeserializer + OwningSerializer + OwningDeserializer,
{
    fn make_startup_pack(account: &Account) -> BorrowingStartupPack {
        BorrowingStartupPack::new(
            T::as_enum(),
            0,
            0,
            &account.proxy_user,
            &account.proxy_zone,
            &account.client_user,
            &account.client_zone,
            (4, 3, 0),
            "d",
            "packe;CS_NEG_REFUSE",
        )
    }

    fn startup(
        account: &Account,
        sock: &mut TcpStream,
        header_buf: &mut Vec<u8>,

        msg_buf: &mut Vec<u8>,
    ) -> Result<(), IrodsError> {
        let msg_len = T::rods_borrowing_ser(&Self::make_startup_pack(account), msg_buf)?;
        let header_len = T::rods_owning_ser(&OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0), header_buf)?;
        let tmp_buf = &mut [0u8; 4];
        sock.write_all(&((header_len as u32).to_be_bytes()))?;
        sock.write_all(&header_buf[..header_len])?;
        sock.write_all(&msg_buf[..msg_len])?;


        sock.read_exact(tmp_buf)?;
        let header_len = u32::from_be_bytes(*tmp_buf) as usize;
        let header: OwningStandardHeader = T::rods_owning_de(Self::read_from_server_uninit(header_len, header_buf, sock)?)?;
        let msg: BorrowingVersion = T::rods_borrowing_de(Self::read_from_server_uninit(header.msg_len, msg_buf, sock)?)?;
        Ok(())
    }
}
