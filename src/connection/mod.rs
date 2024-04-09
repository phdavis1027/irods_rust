#![allow(warnings)]

pub mod authenticate;
pub mod connect;
pub mod pool;
pub mod ssl;
pub mod tcp;

use crate::{
    bosd::{
        BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializable, BorrowingSerializer,
        OwningDeserializble, OwningDeserializer, OwningSerializable, OwningSerializer,
    },
    connection::ssl::{SslConfig, SslConnector},
    fs::DataObjectHandle,
    msg::{
        bin_bytes_buf::BorrowingStrBuf,
        header::{BorrowingHandshakeHeader, MsgType, OwningStandardHeader},
        version::BorrowingVersion,
    },
};

use deadpool::managed::Manager;

use self::{authenticate::Authenticate, connect::Connect};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{
    fmt::Debug,
    io::{self, Cursor, Write},
    marker::PhantomData,
    path::PathBuf,
    time::Duration,
};

const MAX_PASSWORD_LEN: usize = 50;

#[derive(Clone)]
pub struct Account {
    pub client_user: String,
    pub client_zone: String,
    pub proxy_user: String,
    pub proxy_zone: String,
}

// It might seem a bit inflexible to force all bites to come in  through
// the bytes_buf, but it is actually MORE flexible since it doesn't force
// an allocation of a new buffer if one is not needed. Moreover, if the user
// wants to take ownership (e.g., if they want a String out of the bytes) buf,
// they can just use `std::mem::take` to take ownership of the buffer.
pub struct Connection<T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    pub(crate) connector: C,
    account: Account,
    pub header_buf: Vec<u8>,
    pub msg_buf: Vec<u8>,
    pub bytes_buf: Vec<u8>,
    pub error_buf: Vec<u8>,
    // FIXME: Make this a statically sized array
    signature: Vec<u8>,
    phantom_protocol: PhantomData<T>,
}

pub(crate) fn read_from_server<'s, 'r, R>(
    len: usize,
    buf: &'s mut Vec<u8>,
    connector: &'s mut R,
) -> Result<&'r [u8], IrodsError>
where
    R: io::Read + io::Write,
    's: 'r,
{
    if len > buf.len() {
        buf.resize(len, 0);
    }

    connector.read_exact(&mut buf[..len])?;
    Ok(&buf[..len])
}

pub(crate) fn send_owning_msg_and_header<T, S, M>(
    connector: &mut S,
    msg: M,
    msg_type: MsgType,
    int_info: i32,
    msg_buf: &mut Vec<u8>,
    header_buf: &mut Vec<u8>,
) -> Result<(), IrodsError>
where
    T: OwningSerializer,
    S: io::Write,
    M: OwningSerializable,
{
    let msg_len = T::rods_owning_ser(&msg, msg_buf)?;
    let header = OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info);
    let header_len = T::rods_owning_ser(&header, header_buf)?;

    connector.write_all(&(header_len as u32).to_be_bytes())?;
    connector.write_all(&header_buf[..header_len])?;
    connector.write_all(&msg_buf[..msg_len])?;
    Ok(())
}

pub(crate) fn send_borrowing_msg_and_header<'s, 'r, T, S, M>(
    connector: &mut S,
    msg: M,
    msg_type: MsgType,
    int_info: i32,
    msg_buf: &'r mut Vec<u8>,
    header_buf: &'r mut Vec<u8>,
) -> Result<(), IrodsError>
where
    T: BorrowingSerializer + OwningSerializer,
    S: io::Write,
    M: BorrowingSerializable<'s>,
    's: 'r,
{
    let msg_len = T::rods_borrowing_ser(msg, msg_buf)?;
    let header = OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info);
    let header_len = T::rods_owning_ser(&header, header_buf)?;

    connector.write_all(&(header_len as u32).to_be_bytes())?;
    connector.write_all(&header_buf[..header_len])?;
    connector.write_all(&msg_buf[..msg_len])?;
    Ok(())
}

/* std::io::copy does this
pub(crate) fn read_into<R>(
    buf: &mut Vec<u8>,
    len: usize,
    connector: &mut R,
) -> Result<(), IrodsError>
where
    R: io::Read + io::Write,
{
    if len > buf.len() {
        buf.resize(len, 0);
    }

    connector.read_exact(&mut buf[..len])?;
    Ok(())
}
*/

pub(crate) fn read_standard_header<S, T>(
    buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<OwningStandardHeader, IrodsError>
where
    S: io::Read + io::Write,
    T: OwningDeserializer,
{
    connector.read_exact(&mut buf[..4])?;
    let header_len = u32::from_be_bytes(buf[..4].try_into().unwrap()) as usize;

    connector.read_exact(&mut buf[..header_len])?;
    let header: OwningStandardHeader = T::rods_owning_de(&buf[..header_len])?;

    if header.error_len != 0 {}

    if header.bs_len != 0 {}

    if header.int_info != 0 {
        return Err(IrodsError::Other("int_info is not 0".to_string()));
    }

    Ok(header)
}

pub fn send_borrowing_handshake_header<'s, 'r, S, T>(
    connector: &mut S,
    header: BorrowingHandshakeHeader<'s>,
    buf: &'r mut Vec<u8>,
) -> Result<(), IrodsError>
where
    S: std::io::Write,
    T: BorrowingSerializer,
    's: 'r,
{
    let header_len = T::rods_borrowing_ser(header, buf)?;
    connector.write_all(&(header_len as u32).to_be_bytes())?;
    connector.write_all(&buf[..header_len])?;
    Ok(())
}

pub(crate) fn read_borrowing_msg<'s, 'r, S, T, M>(
    len: usize,
    buf: &'s mut Vec<u8>,
    connector: &mut S,
) -> Result<M, IrodsError>
where
    M: BorrowingDeserializable<'r>,
    T: BorrowingDeserializer,
    S: io::Read + io::Write,
    's: 'r,
{
    read_from_server(len, buf, connector)?;
    #[cfg(test)]
    {
        let recv_strfied_msg = std::str::from_utf8(&buf[..len]).unwrap();
        dbg!(recv_strfied_msg);
    }
    T::rods_borrowing_de(&buf[..len])
}

pub(crate) fn read_header_and_borrowing_msg<'s, 'r, S, T, M>(
    msg_buf: &'s mut Vec<u8>,
    header_buf: &'s mut Vec<u8>,
    connector: &mut S,
) -> Result<(OwningStandardHeader, M), IrodsError>
where
    S: io::Read + io::Write,
    T: BorrowingDeserializer + OwningDeserializer,
    M: BorrowingDeserializable<'r>,
    's: 'r,
{
    let header = read_standard_header::<S, T>(header_buf, connector)?;

    let msg = read_borrowing_msg::<S, T, _>(header.msg_len, msg_buf, connector)?;
    Ok((header, msg))
}

pub(crate) fn read_owning_msg<S, T, M>(
    len: usize,
    buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<M, IrodsError>
where
    S: io::Read + io::Write,
    T: OwningDeserializer,
    M: OwningDeserializble,
{
    read_from_server(len, buf, connector)?;
    #[cfg(test)]
    {
        let recv_strfied_msg = std::str::from_utf8(&buf[..len]).unwrap();
        dbg!(recv_strfied_msg);
    }
    T::rods_owning_de(&buf[..len])
}

pub(crate) fn read_header_and_owning_msg<S, T, M>(
    msg_buf: &mut Vec<u8>,
    header_buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<(OwningStandardHeader, M), IrodsError>
where
    S: io::Read + io::Write,
    T: OwningDeserializer,
    M: OwningDeserializble,
{
    let header = read_standard_header::<S, T>(header_buf, connector)?;

    let msg = read_owning_msg::<S, T, _>(header.msg_len, msg_buf, connector)?;

    Ok((header, msg))
}

impl<T, C> Connection<T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    pub(crate) fn new(
        connector: C,
        account: Account,
        header_buf: Vec<u8>,
        msg_buf: Vec<u8>,
        bytes_buf: Vec<u8>,
        error_buf: Vec<u8>,
    ) -> Self {
        Connection {
            connector,
            account,
            header_buf,
            msg_buf,
            bytes_buf,
            error_buf,
            signature: Vec::with_capacity(16),
            phantom_protocol: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    extern crate tokio;

    use super::*;

    use crate::connection::pool::IrodsManager;
    use std::{
        net::{Ipv4Addr, SocketAddr, SocketAddrV4},
        path::PathBuf,
    };

    use deadpool::managed::{Pool, PoolBuilder};
    use deadpool_sync::SyncWrapper;
    use native_tls::TlsConnector;

    use super::{authenticate::NativeAuthenticator, tcp::TcpConnector};
    use crate::bosd::xml::XML;

    #[tokio::test]
    async fn xml_tcp_native_auth() {
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from([172, 18, 0, 3]), 1247));
        let connector = TcpConnector::new(addr);

        let account = super::Account {
            client_user: "rods".into(),
            client_zone: "tempZone".into(),
            proxy_user: "rods".into(),
            proxy_zone: "tempZone".into(),
        };

        let manager: super::pool::IrodsManager<XML, _, _> =
            super::pool::IrodsManager::new(account, authenticator, connector, 30, 5);

        let pool: Pool<IrodsManager<_, _, _>> =
            Pool::builder(manager).max_size(16).build().unwrap();

        let mut conn = pool.get().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn xml_ssl_native_auth() {
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from([172, 18, 0, 3]), 1247));
        let ssl_config = SslConfig::new(
            PathBuf::from("server.crt"),
            "localhost".into(),
            32,
            8,
            16,
            "AES-256-CBC".into(),
        );
        let connector = SslConnector::new(addr, ssl_config);

        let account = super::Account {
            client_user: "rods".into(),
            client_zone: "tempZone".into(),
            proxy_user: "rods".into(),
            proxy_zone: "tempZone".into(),
        };

        let manager: super::pool::IrodsManager<XML, _, _> =
            super::pool::IrodsManager::new(account, authenticator, connector, 30, 5);

        let pool: Pool<IrodsManager<_, _, _>> =
            Pool::builder(manager).max_size(16).build().unwrap();

        let mut conn = pool.get().await.unwrap();
    }
}
