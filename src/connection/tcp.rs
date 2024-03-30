use std::net::{SocketAddr, TcpStream};

use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        xml::XML, BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer,
    },
    msg::{startup_pack::BorrowingStartupPack, version::BorrowingVersion},
};

use super::{
    connect::Connect, read_header_and_borrowing_msg, send_borrowing_msg_and_header, Account,
    Connection,
};

#[derive(Clone)]
pub struct TcpConnector {
    addr: SocketAddr,
}

impl TcpConnector {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

impl<T> Connect<T> for TcpConnector
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningDeserializer + OwningSerializer,
{
    type Transport = TcpStream;

    fn start(
        &self,
        acct: Account,
        mut header_buf: Vec<u8>,
        mut msg_buf: Vec<u8>,
        mut unencoded_buf: Vec<u8>,
        mut encoded_buf: Vec<u8>,
    ) -> Result<Connection<T, Self::Transport>, IrodsError> {
        let mut stream = TcpStream::connect(self.addr)?;

        let startup_pack = BorrowingStartupPack::new(
            T::as_enum(),
            0,
            0,
            &acct.proxy_user,
            &acct.proxy_zone,
            &acct.client_user,
            &acct.client_zone,
            (4, 3, 0),
            "d",
            "packe",
        );

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut stream,
            startup_pack,
            crate::msg::header::MsgType::RodsConnect,
            0,
            &mut msg_buf,
            &mut header_buf,
        )?;

        let (_, version): (_, BorrowingVersion) =
            read_header_and_borrowing_msg::<_, XML, _>(&mut msg_buf, &mut header_buf, &mut stream)?;

        if version.rel_version.0 != 4 {
            return Err(IrodsError::Other("Unsupported server version".into()));
        }

        if version.status < 0 {
            return Err(IrodsError::Other("Server returned an error".into()));
        }

        let connection = Connection::new(
            stream,
            acct,
            header_buf,
            msg_buf,
            unencoded_buf,
            encoded_buf,
        );

        Ok(connection)
    }
}
