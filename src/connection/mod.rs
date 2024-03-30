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
    msg::{
        bin_bytes_buf::BorrowingStrBuf,
        header::{MsgType, OwningStandardHeader},
        version::BorrowingVersion,
    },
};

use deadpool::managed::Manager;

use self::{authenticate::Authenticate, connect::Connect};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use std::{
    fmt::Debug,
    io::{self, Cursor, Write},
    marker::PhantomData,
    time::Duration,
};

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};

static MAX_PASSWORD_LEN: usize = 50;

#[derive(Clone)]
pub struct Account {
    client_user: String,
    client_zone: String,
    proxy_user: String,
    proxy_zone: String,
}

pub struct Connection<T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    connector: C,
    account: Account,
    header_buf: Vec<u8>,
    msg_buf: Vec<u8>,
    unencoded_buf: Vec<u8>,
    encoded_buf: Vec<u8>,
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
    let strfied_msg = std::str::from_utf8(&msg_buf[..msg_len]).unwrap();
    dbg!(strfied_msg);

    let header = OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info);
    let header_len = T::rods_owning_ser(&header, header_buf)?;
    dbg!(header_len);

    let strfied_header = std::str::from_utf8(&header_buf[..header_len]).unwrap();
    dbg!(strfied_header);

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
    let strfied_msg = std::str::from_utf8(&msg_buf[..msg_len]).unwrap();
    dbg!(strfied_msg);

    let header = OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info);
    let header_len = T::rods_owning_ser(&header, header_buf)?;
    dbg!(header_len);

    let strfied_header = std::str::from_utf8(&header_buf[..header_len]).unwrap();
    dbg!(strfied_header);

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
    //UNWRAP: It's 4 bytes long
    let header_len = u32::from_be_bytes(buf[..4].try_into().unwrap()) as usize;

    connector.read_exact(&mut buf[..header_len])?;
    let header: OwningStandardHeader = T::rods_owning_de(&buf[..header_len])?;

    if header.int_info != 0 {
        return Err(IrodsError::Other("int_info is not 0".to_string()));
    }

    Ok(header)
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
    M: OwningDeserializble,
    T: OwningDeserializer,
    S: io::Read + io::Write,
{
    read_from_server(len, buf, connector)?;
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
    pub fn new(
        connector: C,
        account: Account,
        header_buf: Vec<u8>,
        msg_buf: Vec<u8>,
        unencoded_buf: Vec<u8>,
        encoded_buf: Vec<u8>,
    ) -> Self {
        Connection {
            connector,
            account,
            header_buf,
            msg_buf,
            unencoded_buf,
            encoded_buf,
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
    #[ignore]
    async fn xml_tcp_native_auth() {
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from([172, 18, 0, 3]), 1247));
        let connector = TcpConnector::new(addr);

        let account = super::Account {
            client_user: "rods".into(),
            client_zone: "tempZone".into(),
            proxy_user: "".into(),
            proxy_zone: "".into(),
        };

        let manager: super::pool::IrodsManager<XML, _, _> =
            super::pool::IrodsManager::new(account, authenticator, connector, 30, 5);

        let pool: Pool<IrodsManager<_, _, _>> =
            Pool::builder(manager).max_size(16).build().unwrap();

        let mut conn = pool.get().await.unwrap();
    }

    #[tokio::test]
    async fn xml_ssl_native_auth() {
        let authenticator = NativeAuthenticator::new(30, "rods".into());

        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from([172, 18, 0, 3]), 1247));
        let ssl_config = SslConfig::new(PathBuf::from("server.crt"), "172.18.0.3".into());
        let connector = SslConnector::new(addr, ssl_config);

        let account = super::Account {
            client_user: "rods".into(),
            client_zone: "tempZone".into(),
            proxy_user: "".into(),
            proxy_zone: "".into(),
        };

        let manager: super::pool::IrodsManager<XML, _, _> =
            super::pool::IrodsManager::new(account, authenticator, connector, 30, 5);

        let pool: Pool<IrodsManager<_, _, _>> =
            Pool::builder(manager).max_size(16).build().unwrap();

        let mut conn = pool.get().await.unwrap();
    }
}
