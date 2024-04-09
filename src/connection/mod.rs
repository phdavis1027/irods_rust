#![allow(warnings)]

// pub mod authenticate;
// pub mod connect;
// pub mod pool;
// pub mod ssl;
// pub mod tcp;

use crate::{
    bosd::{
        BorrowingDeserializable, BorrowingDeserializer, BorrowingSerializable, BorrowingSerializer,
        OwningDeserializble, OwningDeserializer, OwningSerializable, OwningSerializer,
    },
    msg::header::{MsgType, OwningStandardHeader}, //connection::ssl::{SslConfig, SslConnector},
                                                  // fs::DataObjectHandle,
                                                  // msg::{
                                                  //     bin_bytes_buf::BorrowingStrBuf,
                                                  //     header::{BorrowingHandshakeHeader, MsgType, OwningStandardHeader},
                                                  //     version::BorrowingVersion,
                                                  // },
};

use futures::{future::FutureExt, TryFutureExt};

use deadpool::managed::Manager;

// use self::{authenticate::Authenticate, connect::Connect};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::{
    fmt::Debug,
    io::{self, Cursor, Read},
    marker::PhantomData,
    path::PathBuf,
    time::Duration,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

pub(crate) async fn chain_write_all<R>(buf: &[u8], mut connector: R) -> Result<R, IrodsError>
where
    R: tokio::io::AsyncWrite + Unpin,
{
    connector.write_all(buf).await?;
    Ok(connector)
}

pub(crate) async fn chain_read_from_server<'s, 'r, R>(
    len: usize,
    buf: &'s mut Vec<u8>,
    mut connector: R,
) -> Result<(&'s mut Vec<u8>, R), IrodsError>
where
    R: tokio::io::AsyncRead + Unpin,
    's: 'r,
{
    tokio::io::copy(&mut (&mut connector).take(len as u64), buf).await?;
    Ok((buf, connector))
}

pub(crate) async fn read_standard_header<S, T>(
    buf: &mut Vec<u8>,
    connector: S,
) -> Result<(OwningStandardHeader, &mut Vec<u8>, S), IrodsError>
where
    S: tokio::io::AsyncRead + Unpin,
    T: OwningDeserializer,
{
    Ok(chain_read_from_server(4, buf, connector)
        .and_then(|(b, mut c)| async {
            let header_len = u32::from_be_bytes(b[..4].try_into().unwrap()) as usize;
            chain_read_from_server(header_len, b, &mut c).await?;
            Ok((b, c, header_len))
        })
        .and_then(|(b, c, hl)| async move {
            Ok((T::rods_owning_de::<OwningStandardHeader>(&b[..hl])?, b, c))
        })
        .await?)
}

pub(crate) async fn send_standard_header<S, T>(
    header: OwningStandardHeader,
    buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<(), IrodsError>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: OwningSerializer,
{
    T::rods_owning_ser(&header, buf)?;
    chain_write_all(&buf, connector).await?;
    Ok(())
}

pub(crate) async fn read_owning_msg<S, T, M>(
    len: usize,
    buf: &mut Vec<u8>,
    connector: S,
) -> Result<(M, &mut Vec<u8>, S), IrodsError>
where
    S: tokio::io::AsyncRead + Unpin,
    T: OwningDeserializer,
    M: OwningDeserializble,
{
    Ok(chain_read_from_server(len, buf, connector)
        .and_then(|(b, c)| async {
            let msg = T::rods_owning_de(&b[..len])?;

            Ok((msg, b, c))
        })
        .await?)
}

pub(crate) async fn read_borrowing_msg<'s, 'r, S, T, M>(
    len: usize,
    buf: &'s mut Vec<u8>,
    connector: &mut S,
) -> Result<M, IrodsError>
where
    M: BorrowingDeserializable<'r>,
    T: BorrowingDeserializer,
    S: tokio::io::AsyncRead + Unpin,
    's: 'r,
{
    chain_read_from_server(len, buf, connector)
        .and_then(|(b, _)| async { T::rods_borrowing_de(&b[..len]) })
        .await
}

pub(crate) async fn send_owning_msg<S, T, M>(
    msg: M,
    buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<(), IrodsError>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: OwningSerializer,
    M: OwningSerializable,
{
    T::rods_owning_ser(&msg, buf)?;
    chain_write_all(&buf, connector).await?;
    Ok(())
}

pub(crate) async fn send_borrowing_msg<'r, 's, S, T, M>(
    msg: M,
    buf: &'s mut Vec<u8>,
    connector: &'r mut S,
) -> Result<(), IrodsError>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: BorrowingSerializer,
    M: BorrowingSerializable<'s>,
{
    T::rods_borrowing_ser(msg, buf)?;
    chain_write_all(&buf, connector).await?;
    Ok(())
}

pub(crate) async fn send_header_then_owning_msg<S, T, M>(
    msg: &M,
    msg_type: MsgType,
    int_info: i32,
    header_buf: &mut Vec<u8>,
    msg_buf: &mut Vec<u8>,
    connector: &mut S,
) -> Result<(), IrodsError>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: OwningSerializer,
    M: OwningSerializable,
{
    let msg_len = T::rods_owning_ser(msg, msg_buf)?;
    let header_len = T::rods_owning_ser(
        &OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info),
        header_buf,
    )?;

    chain_write_all(&(header_len as u32).to_be_bytes(), connector)
        .and_then(|c| chain_write_all(header_buf, c))
        .and_then(|c| chain_write_all(msg_buf, c))
        .await?;

    Ok(())
}

pub(crate) async fn send_header_then_borrowing_msg<'r, 's, S, T, M>(
    msg: M,
    msg_type: MsgType,
    int_info: i32,
    header_buf: &mut Vec<u8>,
    msg_buf: &'s mut Vec<u8>,
    connector: &'r mut S,
) -> Result<(), IrodsError>
where
    S: tokio::io::AsyncWrite + Unpin,
    T: BorrowingSerializer + OwningSerializer,
    M: BorrowingSerializable<'s>,
{
    let msg_len = T::rods_borrowing_ser(msg, msg_buf)?;
    let header_len = T::rods_owning_ser(
        &OwningStandardHeader::new(msg_type, msg_len, 0, 0, int_info),
        header_buf,
    )?;

    chain_write_all(&(header_len as u32).to_be_bytes(), connector)
        .and_then(|c| chain_write_all(header_buf, c))
        .and_then(|c| chain_write_all(msg_buf, c))
        .await?;

    Ok(())
}

pub(crate) async fn read_header_then_owning_msg<'buf, S, T, M>(
    header_buf: &'buf mut Vec<u8>,
    msg_buf: &'buf mut Vec<u8>,
    bytes_buf: &'buf mut Vec<u8>,
    error_buf: &'buf mut Vec<u8>,
    mut connector: S,
) -> Result<
    (
        OwningStandardHeader,
        M,
        &'buf mut Vec<u8>,
        &'buf mut Vec<u8>,
        &'buf mut Vec<u8>,
        &'buf mut Vec<u8>,
        S,
    ),
    IrodsError,
>
where
    S: tokio::io::AsyncRead + Unpin,
    T: OwningDeserializer,
    M: OwningDeserializble,
{
    let (msg, header) = read_standard_header::<&mut S, T>(header_buf, &mut connector)
        .and_then(|(header, hb, c)| async {
            let msg_len = header.msg_len as usize;
            let bs_len = header.bs_len as usize;
            let error_len = header.error_len as usize;

            let (msg, mb, c) = read_owning_msg::<&mut S, T, M>(msg_len, msg_buf, c)
                .and_then(|(msg, mb, mut c)| async {
                    chain_read_from_server(bs_len, bytes_buf, &mut c)
                        .and_then(|(_, c)| chain_read_from_server(error_len, error_buf, c))
                        .await?;
                    Ok((msg, mb, c))
                })
                .await?;
            Ok((msg, header))
        })
        .await?;

    Ok((
        header, msg, msg_buf, bytes_buf, error_buf, header_buf, connector,
    ))
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
