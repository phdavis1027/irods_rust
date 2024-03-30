use std::{
    fs::File,
    io::BufReader,
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::Arc,
};

use native_tls::{Certificate, TlsStream};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        xml::XML, BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer,
    },
    common::CsNegPolicy,
    connection::read_header_and_owning_msg,
    msg::{cs_neg::OwningCsNeg, startup_pack::BorrowingStartupPack, version::BorrowingVersion},
};

use super::{
    connect::Connect, read_header_and_borrowing_msg, send_borrowing_msg_and_header, Connection,
};

pub struct SslConnector {
    inner: Arc<SslConnectorInner>,
}

pub struct SslConfig {
    pub cert_file: PathBuf,
    pub domain: String,
}

impl SslConfig {
    pub fn new(cert_file: PathBuf, domain: String) -> Self {
        Self { cert_file, domain }
    }
}

pub struct SslConnectorInner {
    pub config: SslConfig,
    pub addr: SocketAddr,
}

impl Clone for SslConnector {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl SslConnector {
    pub fn new(addr: SocketAddr, config: SslConfig) -> Self {
        Self {
            inner: Arc::new(SslConnectorInner { config, addr }),
        }
    }
}

impl<T> Connect<T> for SslConnector
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningDeserializer + OwningSerializer,
{
    type Transport = TlsStream<TcpStream>;

    fn start(
        &self,
        acct: super::Account,
        mut header_buf: Vec<u8>,
        mut msg_buf: Vec<u8>,
        mut unencoded_buf: Vec<u8>,
        mut encoded_buf: Vec<u8>,
    ) -> Result<super::Connection<T, Self::Transport>, IrodsError> {
        let mut stream = TcpStream::connect(self.inner.addr)?;

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
            "packe;request_server_negotiation",
        );

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut stream,
            startup_pack,
            crate::msg::header::MsgType::RodsConnect,
            0,
            &mut msg_buf,
            &mut header_buf,
        )?;

        let (_, cs_neg): (_, OwningCsNeg) =
            read_header_and_owning_msg::<_, XML, _>(&mut msg_buf, &mut header_buf, &mut stream)?;

        if cs_neg.status != 1 || cs_neg.result != CsNegPolicy::CS_NEG_REQUIRE {
            // We can't accept the connection. Send a message saying that
        }

        todo!()
    }
}
