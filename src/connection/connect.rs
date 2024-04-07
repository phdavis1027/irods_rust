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
    ) -> Result<Connection<T, Self::Transport>, IrodsError>;

    fn connect(&self, acct: Account) -> Result<Connection<T, Self::Transport>, IrodsError> {
        let mut header_buf = vec![0; HEADER_BUF_START_SIZE];
        let mut msg_buf = vec![0; MSG_BUF_START_SIZE];

        self.start(acct, header_buf, msg_buf)
    }
}
