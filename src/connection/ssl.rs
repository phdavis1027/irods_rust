use std::{
    fs::File,
    io::{BufReader, Read, Write},
    net::{SocketAddr, TcpStream},
    path::PathBuf,
    sync::Arc,
};

use native_tls::{Certificate, TlsConnector, TlsStream};
use rand::{random, RngCore};
use rods_prot_msg::error::errors::IrodsError;

use crate::{
    bosd::{
        xml::XML, BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer,
    },
    common::{CsNegPolicy, CsNegResult},
    connection::{read_header_and_owning_msg, send_owning_msg_and_header},
    msg::{
        cs_neg::{OwningClientCsNeg, OwningServerCsNeg},
        header::BorrowingHandshakeHeader,
        startup_pack::BorrowingStartupPack,
        version::BorrowingVersion,
    },
};

use super::{
    connect::Connect, read_header_and_borrowing_msg, send_borrowing_handshake_header,
    send_borrowing_msg_and_header, Connection,
};

pub struct SslConnector {
    inner: Arc<SslConnectorInner>,
}

pub struct SslConfig {
    pub cert_file: PathBuf,
    pub domain: String,
    pub key_size: usize,
    pub salt_size: usize,
    pub hash_rounds: usize,
    algorithm: String,
}

impl SslConfig {
    pub fn new(
        cert_file: PathBuf,
        domain: String,
        key_size: usize,
        salt_size: usize,
        hash_rounds: usize,
        algorithm: String,
    ) -> Self {
        Self {
            cert_file,
            domain,
            key_size,
            salt_size,
            hash_rounds,
            algorithm,
        }
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
        mut bytes_buf: Vec<u8>,
        mut error_buf: Vec<u8>,
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
            "request_server_negotiation;",
        );

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut stream,
            startup_pack,
            crate::msg::header::MsgType::RodsConnect,
            0,
            &mut msg_buf,
            &mut header_buf,
        )?;

        let (_, cs_neg): (_, OwningServerCsNeg) =
            read_header_and_owning_msg::<_, XML, _>(&mut msg_buf, &mut header_buf, &mut stream)?;

        if cs_neg.status != 1 || cs_neg.result != CsNegPolicy::CS_NEG_REQUIRE {
            // We can't accept the connection. Send a message saying that
        }

        let client_cs_neg = OwningClientCsNeg::new(1, CsNegResult::CS_NEG_USE_SSL);
        send_owning_msg_and_header::<XML, _, _>(
            &mut stream,
            client_cs_neg,
            crate::msg::header::MsgType::RodsCsNeg,
            0,
            &mut msg_buf,
            &mut header_buf,
        );

        let (_, version): (_, BorrowingVersion) =
            read_header_and_borrowing_msg::<_, XML, _>(&mut msg_buf, &mut header_buf, &mut stream)?;

        // Tls only zone from here on
        let connector = TlsConnector::builder()
            .add_root_certificate(Certificate::from_pem(&std::fs::read(
                self.inner.config.cert_file.clone(),
            )?)?)
            .danger_accept_invalid_certs(true) // FIXME: Bad, only for testing
            .build()?;

        let mut stream = connector.connect(&self.inner.config.domain, stream)?;

        let ssl_config_header = BorrowingHandshakeHeader::new(
            &self.inner.config.algorithm,
            self.inner.config.key_size,
            self.inner.config.salt_size,
            self.inner.config.hash_rounds,
        );
        send_borrowing_handshake_header::<_, T>(&mut stream, ssl_config_header, &mut msg_buf)?;

        let shared_secret = &mut msg_buf[..self.inner.config.key_size]; // FIXME: Generate a real shared secret
        rand::thread_rng().fill_bytes(shared_secret);

        let shared_secret_header = BorrowingHandshakeHeader::new(
            "SHARED_SECRET",
            usize::from_be_bytes((&shared_secret[..8]).try_into().unwrap()),
            0,
            0,
        );
        send_borrowing_handshake_header::<_, T>(&mut stream, shared_secret_header, &mut msg_buf)?;

        Ok(Connection::new(
            stream, acct, header_buf, msg_buf, error_buf, bytes_buf,
        ))
    }
}
