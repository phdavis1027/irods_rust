#![allow(warnings)]

pub mod connect;
pub mod ssl;

use std::marker::PhantomData;

use futures::future::TryFutureExt;
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;
use std::io::Cursor;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{
    bosd::{Deserializable, ProtocolEncoding, Serialiazable},
    common::CsNegResult,
    msg::{
        cs_neg::{ClientCsNeg, ServerCsNeg},
        header::{MsgType, StandardHeader},
        startup_pack::{self, StartupPack},
    },
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
pub struct UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    resources: ResourceBundle<C>,
    account: Account,
    signature: Vec<u8>,
    phantom_protocol: PhantomData<T>,
}

pub struct ResourceBundleInner<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub header_buf: Vec<u8>,
    pub msg_buf: Vec<u8>,
    pub bytes_buf: Vec<u8>,
    pub error_buf: Vec<u8>,
    pub connector: S,
}

#[repr(transparent)]
pub struct ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    inner: Box<ResourceBundleInner<S>>,
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub fn new(connector: S) -> Self {
        Self {
            inner: Box::new(ResourceBundleInner {
                header_buf: Vec::new(),
                msg_buf: Vec::new(),
                bytes_buf: Vec::new(),
                error_buf: Vec::new(),
                connector,
            }),
        }
    }
}

impl<S> ResourceBundle<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    async fn send_header_len(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner
            .connector
            .write_all(&(len as u32).to_be_bytes())
            .await?;
        Ok(inner)
    }

    async fn send_from_msg_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner.connector.write_all(&mut inner.msg_buf[..len]).await?;
        Ok(inner)
    }

    async fn send_from_header_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        inner
            .connector
            .write_all(&mut inner.header_buf[..len])
            .await?;
        Ok(inner)
    }

    async fn read_to_msg_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.msg_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_header_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.header_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_bytes_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.bytes_buf),
        )
        .await?;
        Ok(inner)
    }

    async fn read_to_error_buf(
        inner: &mut ResourceBundleInner<S>,
        len: usize,
    ) -> Result<&mut ResourceBundleInner<S>, IrodsError> {
        tokio::io::copy(
            &mut (&mut inner.connector).take(len as u64),
            &mut Cursor::new(&mut inner.error_buf),
        )
        .await?;
        Ok(inner)
    }

    pub(crate) async fn read_standard_header<T>(
        &mut self,
    ) -> Result<(StandardHeader, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
    {
        let header = Self::read_to_header_buf(&mut self.inner, 4)
            .and_then(|inner| async {
                let header_len =
                    u32::from_be_bytes(inner.header_buf[..4].try_into().unwrap()) as usize;

                let inner = Self::read_to_header_buf(inner, header_len).await?;

                Ok(T::decode(&inner.header_buf[..header_len])?)
            })
            .await?;

        Ok((header, self))
    }

    pub(crate) async fn read_msg<T, M>(&mut self, len: usize) -> Result<(M, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        let msg = async {
            let inner = Self::read_to_msg_buf(&mut self.inner, len).await?;
            Ok::<_, IrodsError>(T::decode(&inner.msg_buf[..len])?)
        }
        .await?;

        Ok((msg, self))
    }

    pub(crate) async fn send_standard_header<T>(
        &mut self,
        header: StandardHeader,
    ) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
    {
        let len = T::encode(&header, &mut self.inner.header_buf)?;

        Self::send_header_len(&mut self.inner, len)
            .and_then(|inner| Self::send_from_header_buf(inner, len))
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_msg<T, M>(&mut self, msg: M) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        let len = T::encode(&msg, &mut self.inner.msg_buf)?;

        Self::send_from_msg_buf(&mut self.inner, len).await?;

        Ok(self)
    }

    pub(crate) async fn get_header_and_msg<T, M>(
        &mut self,
    ) -> Result<(StandardHeader, M, &mut Self), IrodsError>
    where
        T: ProtocolEncoding,
        M: Deserializable,
    {
        let (header, _) = self.read_standard_header::<T>().await?;

        let (msg, this) = self.read_msg::<T, M>(header.msg_len as usize).await?;

        Self::read_to_bytes_buf(&mut this.inner, header.bs_len as usize)
            .and_then(|inner| Self::read_to_error_buf(inner, header.error_len as usize))
            .await?;

        Ok((header, msg, self))
    }

    pub(crate) async fn send_header_then_msg<T, M>(
        &mut self,
        msg: &M,
        msg_type: MsgType,
        int_info: i32,
    ) -> Result<&mut Self, IrodsError>
    where
        T: ProtocolEncoding,
        M: Serialiazable,
    {
        let msg_len = T::encode(msg, &mut self.inner.msg_buf)?;

        let header = StandardHeader::new(msg_type, msg_len, 0, 0, int_info);

        Self::send_standard_header::<T>(self, header)
            .and_then(|this| Self::send_from_msg_buf(&mut this.inner, msg_len))
            .await?;

        Ok(self)
    }
}

impl<T, C> UnauthenticatedConnection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    pub(crate) fn new(account: Account, resources: ResourceBundle<C>) -> Self {
        Self {
            resources,
            account,
            signature: Vec::with_capacity(16),
            phantom_protocol: PhantomData,
        }
    }

    pub(crate) async fn send_startup_pack(
        &mut self,
        reconnect_flag: u32,
        connect_cnt: u32,
        proxy_user: String,
        proxy_zone: String,
        client_user: String,
        client_zone: String,
        rel_version: (u8, u8, u8),
        option: String,
    ) -> Result<&mut Self, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &StartupPack::new(
                    T::as_enum(),
                    reconnect_flag,
                    connect_cnt,
                    proxy_user,
                    proxy_zone,
                    client_user,
                    client_zone,
                    rel_version,
                    option,
                ),
                MsgType::RodsConnect,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_server_cs_neg(
        &mut self,
    ) -> Result<(StandardHeader, ServerCsNeg, &mut Self), IrodsError> {
        let (header, msg, _) = self.resources.get_header_and_msg::<T, _>().await?;

        Ok((header, msg, self))
    }

    pub(crate) async fn send_use_ssl(&mut self) -> Result<&mut Self, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_SSL),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_use_tcp(&mut self) -> Result<&mut Self, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(1, CsNegResult::CS_NEG_USE_TCP),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn send_negotiation_failed(&mut self) -> Result<&mut Self, IrodsError> {
        self.resources
            .send_header_then_msg::<T, _>(
                &ClientCsNeg::new(0, CsNegResult::CS_NEG_FAILURE),
                MsgType::RodsCsNeg,
                0,
            )
            .await?;

        Ok(self)
    }

    pub(crate) async fn get_version(&mut self) -> Result<(StandardHeader, &mut Self), IrodsError> {
        let (header, _, _) = self
            .resources
            .get_header_and_msg::<T, StandardHeader>()
            .await?;

        Ok((header, self))
    }
}

#[cfg(test)]
mod test {}
