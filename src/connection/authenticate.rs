use std::io::{self, Cursor, Read, Write};

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use futures::TryFutureExt;
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;
use serde::Deserialize;
use std::borrow::{BorrowMut, Cow};

use crate::bosd::xml::XML;
use crate::bosd::ProtocolEncoding;
use crate::common::{self, APN};
use crate::connection::MAX_PASSWORD_LEN;
use crate::msg::bin_bytes_buf::BinBytesBuf;
use crate::msg::header::MsgType;

use super::{Connection, UnauthenticatedConnection};

#[derive(Deserialize)]
pub struct AuthAgentAuthResponse<'s> {
    #[serde(borrow)]
    a_ttl: Cow<'s, str>,

    #[serde(borrow)]
    force_password_prompt: Cow<'s, str>,

    #[serde(borrow)]
    next_operation: Cow<'s, str>,

    #[serde(borrow)]
    request_result: Cow<'s, str>,

    #[serde(borrow)]
    scheme: Cow<'s, str>,

    #[serde(borrow)]
    user_name: Cow<'s, str>,

    #[serde(borrow)]
    zone_name: Cow<'s, str>,
}

pub trait Authenticate<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
{
    fn authenticate(
        &self,
        conn: UnauthenticatedConnection<T, C>,
    ) -> impl std::future::Future<Output = Result<Connection<T, C>, IrodsError>> + std::marker::Send;
}

pub struct NativeAuthenticator {
    pub a_ttl: u32,
    pub password: String,
    pub b64_engine: GeneralPurpose,
}

impl NativeAuthenticator {
    pub fn new(a_ttl: u32, password: String) -> Self {
        Self {
            a_ttl,
            password,
            b64_engine: Self::create_base64_engine(),
        }
    }
}

impl NativeAuthenticator {
    pub(crate) fn create_base64_engine() -> GeneralPurpose {
        let cfg = GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true);
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }
}

impl<T, C> Authenticate<T, C> for NativeAuthenticator
where
    T: ProtocolEncoding + Send,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin,
{
    async fn authenticate(
        &self,
        conn: UnauthenticatedConnection<T, C>,
    ) -> Result<Connection<T, C>, IrodsError> {
        conn.send_auth_request(self)
            .and_then(|conn| conn.get_auth_response())
            .and_then(|(challenge, mut conn)| async move {
                let payload_len = self
                    .b64_engine
                    .decode_slice(
                        challenge.buf.as_bytes(),
                        conn.inner.resources.inner.error_buf.as_mut_slice(),
                    )
                    .map_err(|e| {
                        IrodsError::Other(format!("Failed to decode challenge: {:?}", e))
                    })?;

                let request_result =
                    std::str::from_utf8(&conn.inner.resources.inner.error_buf[..payload_len - 1])
                        .unwrap();

                let request_result = serde_json::from_str::<AuthAgentAuthResponse>(request_result)
                    .map_err(|e| IrodsError::Other(format!("Failed to parse challenge: {:?}", e)))?
                    .request_result;

                let mut signature = Vec::with_capacity(16);
                request_result.as_bytes().iter().take(16).for_each(|c| {
                    signature.push(*c);
                });

                let mut digest = Md5::new();
                digest.update(request_result.as_bytes());

                let pad_buf = &mut conn.inner.resources.inner.bytes_buf[..MAX_PASSWORD_LEN];
                pad_buf.fill(0);

                for (i, c) in self.password.as_bytes().iter().enumerate() {
                    pad_buf[i] = *c;
                }

                digest.update(pad_buf);

                let mut unencoded_cursor = Cursor::new(&mut conn.inner.resources.inner.bytes_buf);

                write!(
                    unencoded_cursor,
                    r#"
                    {{
                        "a_ttl": {0},
                        "force_password_prompt": "true",
                        "next_operation": "auth_agent_auth_response",
                        "scheme": "native",
                        "user_name": "{1}",
                        "zone_name": "{2}",
                        "digest": "{3}"
                    }}"#,
                    self.a_ttl,
                    conn.inner.account.client_user,
                    conn.inner.account.client_zone,
                    STANDARD.encode(digest.finalize())
                )?;

                let unencoded_len = unencoded_cursor.position() as usize;
                conn.inner
                    .resources
                    .inner
                    .error_buf
                    .resize(4 * unencoded_len / 3 + 4, 0);

                let payload_len = self
                    .b64_engine
                    .encode_slice(
                        &conn.inner.resources.inner.bytes_buf[..unencoded_len],
                        conn.inner.resources.inner.error_buf.as_mut_slice(),
                    )
                    .map_err(|e| IrodsError::Other(format!("Failed to encode payload: {:?}", e)))?;

                let encoded =
                    std::str::from_utf8(&conn.inner.resources.inner.error_buf[..payload_len])
                        .map_err(|e| {
                            IrodsError::Other(format!(
                                "Failed to convert payload to string: {:?}",
                                e
                            ))
                        })?;

                let buf = BinBytesBuf::new(encoded);

                conn.inner
                    .resources
                    .send_header_then_msg::<T, _>(
                        &buf,
                        MsgType::RodsApiReq,
                        APN::Authentication as i32,
                    )
                    .await?;

                Ok((signature, conn))
            })
            .and_then(|(signature, conn)| {
                conn.get_auth_response()
                    .and_then(|(_, conn)| async move { Ok(conn.into_authenticated(signature)) })
            })
            .await
    }
}
