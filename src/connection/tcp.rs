use std::{
    io::{self, Read, Write},
    marker::PhantomData,
    net::TcpStream,
    time::Duration,
};

use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        BorrowingDeserializer, BorrowingSerializer, IrodsProtocol, OwningDeserializer,
        OwningSerializer,
    },
    msg::{
        header::{MsgType, OwningStandardHeader},
        startup_pack::BorrowingStartupPack,
        version::BorrowingVersion,
    },
};

use super::{authenticate::Authenticate, Account, ConnConfig, Connection, CsNegPolicy};

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

#[cfg(test)]
impl ConnConfig<TcpStream> {
    fn test_config() -> Self {
        Self::new(
            8092,
            Duration::from_secs(5),
            Duration::from_secs(5),
            "172.27.0.4".into(),
            1247,
            30,
        )
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

impl<T, A> Connection<T, TcpStream, A>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    A: Authenticate<T, TcpStream>,
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
        let header_len = T::rods_owning_ser(
            &OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0),
            header_buf,
        )?;
        let header_len_buf = (header_len as u32).to_be_bytes();
        sock.write_all(&header_len_buf)?;
        println!("Wrote header len: {}", header_len as u32);
        println!(
            "Header len as bytes: {:?}",
            (header_len as u32).to_be_bytes()
        );
        sock.write_all(&header_buf[..header_len])?;
        println!("Wrote header");
        sock.write_all(&msg_buf[..msg_len])?;
        println!("Wrote msg");

        let mut tmp_buf = [0u8; 4];
        sock.read(&mut tmp_buf)?;
        dbg!(tmp_buf);
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader =
            T::rods_owning_de(Self::read_from_server_uninit(header_len, header_buf, sock)?)?;
        let msg: BorrowingVersion = T::rods_borrowing_de(Self::read_from_server_uninit(
            header.msg_len,
            msg_buf,
            sock,
        )?)?;

        Ok(())
    }

    pub fn new(account: &Account, config: &ConnConfig<TcpStream>) -> Result<Self, IrodsError> {
        let mut socket = TcpStream::connect(&config.addr)?;

        let mut header_buf = vec![0u8; HEADER_BUF_START_SIZE];
        let mut msg_buf = vec![0u8; MSG_BUF_START_SIZE];
        let mut scratch_buf = vec![0u8; SCRATCH_BUF_START_SIZE];

        Self::startup(account, &mut socket, &mut header_buf, &mut msg_buf)?;
        Self::authenticate(
            account,
            config,
            &mut socket,
            &mut header_buf,
            &mut msg_buf,
            &mut scratch_buf,
        )?;

        Ok(Self {
            account: account.clone(),
            config: config.clone(),
            header_buf,
            msg_buf,
            scratch_buf,
            socket,
            signature: Vec::new(),
            phantom_protocol: PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::bosd::xml::XML;

    use super::*;

    #[test]
    #[ignore]
    fn tcp_connects_correctly() {
        let account = Account::test_account();
        let config = ConnConfig::test_config();
        let conn: Connection<XML, TcpStream> = Connection::new(&account, &config).unwrap();
    }
}
