use std::{
    io,
    net::{SocketAddr, TcpStream},
};

use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        xml::XML, BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer,
    },
    msg::{
        header::OwningStandardHeader, startup_pack::BorrowingStartupPack, version::BorrowingVersion,
    },
};

use super::{
    read_header_and_borrowing_msg, send_borrowing_msg_and_header, send_owning_msg_and_header,
    Account, Connection,
};

static HEADER_BUF_START_SIZE: usize = 512;
static MSG_BUF_START_SIZE: usize = 2048;
static UNENCODED_BUF_START_SIZE: usize = 8092;
static ENCODED_BUF_START_SIZE: usize = 8092;

pub trait Connect<T>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningDeserializer + OwningSerializer,
{
    type Transport: io::Read + io::Write;

    fn start(
        &self,
        acct: Account,
        header_buf: Vec<u8>,
        msg_buf: Vec<u8>,
        unencoded_buf: Vec<u8>,
        encoded_buf: Vec<u8>,
    ) -> Result<Connection<T, Self::Transport>, IrodsError>;

    fn connect(&self, acct: Account) -> Result<Connection<T, Self::Transport>, IrodsError> {
        let mut header_buf = vec![0; HEADER_BUF_START_SIZE];
        let mut msg_buf = vec![0; MSG_BUF_START_SIZE];
        let mut unencoded_buf = vec![0; UNENCODED_BUF_START_SIZE];
        let mut encoded_buf = vec![0; ENCODED_BUF_START_SIZE];

        self.start(acct, header_buf, msg_buf, unencoded_buf, encoded_buf)
    }
}
pub struct TcpConnector {
    addr: SocketAddr,
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
            "packe;CS_NEG_REFUSE",
        );

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut stream,
            startup_pack,
            crate::msg::header::MsgType::RodsConnect,
            0,
            &mut msg_buf,
            &mut header_buf,
        )?;

        let (_, version) = read_header_and_borrowing_msg::<_, XML, BorrowingVersion>(
            &mut msg_buf,
            &mut header_buf,
            &mut stream,
        )?;

        if version.rel_version.0 < 4 {
            return Err(IrodsError::Other("Server version is too low".into()));
        }

        let connection = Connection::new(
            stream,
            acct.clone(),
            header_buf,
            msg_buf,
            unencoded_buf,
            encoded_buf,
        );

        Ok(connection)
    }
}
