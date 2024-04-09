use std::io::{self, Cursor, Read, Write};
use std::sync::{Arc, Mutex};

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;
use serde::Deserialize;
use std::borrow::{BorrowMut, Cow};

use crate::bosd::xml::XML;
use crate::common::{self, APN};
use crate::connection::{
    read_from_server, read_header_and_borrowing_msg, send_borrowing_msg_and_header,
    send_owning_msg_and_header,
};
use crate::{
    bosd::{BorrowingDeserializer, BorrowingSerializer, OwningDeserializer, OwningSerializer},
    connection::{Connection, MAX_PASSWORD_LEN},
    msg::{
        bin_bytes_buf::BorrowingStrBuf,
        header::{MsgType, OwningStandardHeader},
    },
};

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
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    type Output;

    fn authenticate(&self, conn: &mut Connection<T, C>) -> Result<Self::Output, IrodsError>;
}

pub struct NativeAuthenticator {
    a_ttl: u32,
    password: String,
}

impl NativeAuthenticator {
    pub fn new(a_ttl: u32, password: String) -> Self {
        Self { a_ttl, password }
    }
}

impl NativeAuthenticator {
    fn create_base64_engine() -> GeneralPurpose {
        let cfg = GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true);
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }
}

impl<T, C> Authenticate<T, C> for NativeAuthenticator
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    type Output = Vec<u8>;

    fn authenticate(&self, conn: &mut Connection<T, C>) -> Result<Self::Output, IrodsError> {
        let mut signature = Vec::with_capacity(16);

        let b64_engine = Self::create_base64_engine();

        // UNSAFE: Connection buffers are always initialized with
        // at least enough space for the payload
        let mut unencoded_cursor = Cursor::new(&mut conn.bytes_buf);

        write!(
            unencoded_cursor,
            r##"
        {{
            "a_ttl":"{0}",
            "force_password_prompt": "true",
            "next_operation": "auth_agent_auth_request",
            "scheme": "native",
            "user_name": "{1}",
            "zone_name": "{2}"
        }}
        "##,
            self.a_ttl, conn.account.client_user, conn.account.client_zone
        )?;

        let unencoded_len = unencoded_cursor.position() as usize;
        conn.error_buf.resize(4 * (unencoded_len / 3) + 1, 0); // Make sure we have enough room
                                                               // to store the encoded string.
        let payload_len = b64_engine
            .encode_slice(
                &conn.bytes_buf[..unencoded_len], // UNSAFE: The cursor guarantees we have enough space
                conn.error_buf.as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks.".into()))?;

        // UNSAFE: Base64 is always valid UTF-8
        let encoded_str = unsafe { std::str::from_utf8_unchecked(&conn.error_buf[..payload_len]) };
        let str_buf = BorrowingStrBuf::new(encoded_str);

        send_borrowing_msg_and_header::<T, _, _>(
            &mut conn.connector,
            str_buf,
            MsgType::RodsApiReq,
            APN::Authentication as i32,
            &mut conn.msg_buf,
            &mut conn.header_buf,
        );

        let (_, challenge) = read_header_and_borrowing_msg::<_, T, BorrowingStrBuf>(
            &mut conn.msg_buf,
            &mut conn.header_buf,
            &mut conn.connector,
        )?;

        // decode the challenge buffer
        // we know the challenge buffer is long enough to hold the decoded value
        // because base64 makes strings lsonger
        let payload_len = b64_engine
            .decode_slice(challenge.buf.as_bytes(), &mut conn.bytes_buf)
            .map_err(|e| IrodsError::Other(format!("Failed to decode challenge: {:?}", e)))?;

        let challenge_str = unsafe {
            std::str::from_utf8_unchecked(conn.error_buf.as_slice().get(..payload_len - 1).unwrap())
        };

        let request_result = serde_json::from_str::<AuthAgentAuthResponse>(challenge_str)
            .map_err(|e| IrodsError::Other(format!("Failed to parse challenge: {:?}", e)))?
            .request_result;

        // Can't use copy because Cow doesn't give mutable access to borrowed values
        for (i, c) in request_result.as_bytes().iter().take(16).enumerate() {
            signature.push(*c);
        }

        // This is fine because the Md5 state only
        // takes up a 4-length array of u32s
        let mut digest = Md5::new();
        digest.update(request_result.as_bytes());

        // Briefly repurpose the unencoded buf
        let mut pad_buf = &mut conn.bytes_buf[..MAX_PASSWORD_LEN];
        pad_buf.fill(0);
        // TODO: There simply must be a way to use std::io::copy here
        for (i, c) in self.password.as_bytes().iter().enumerate() {
            pad_buf[i] = *c;
        }
        digest.update(pad_buf); //BORROWEND: pad_buf

        let mut unencoded_cursor = Cursor::new(&mut conn.bytes_buf);
        // TODO: Some slice kung fu to make get rid of the allocation inucrred
        // by STANDARD.encode
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
            conn.account.client_user,
            conn.account.client_zone,
            STANDARD.encode(digest.finalize())
        );

        let unencoded_len = unencoded_cursor.position() as usize;
        conn.error_buf.resize(5 * (unencoded_len / 3) + 1, 0); // Make sure we have enough room
        let payload_len = b64_engine
            .encode_slice(
                &unencoded_cursor.get_mut()[..unencoded_len],
                conn.error_buf.as_mut_slice(),
            )
            .map_err(|e| {
                println!("Error: {:?}", e);
                IrodsError::Other("FIXME: This sucks".into())
            })?;

        let encoded_str = unsafe { std::str::from_utf8_unchecked(&conn.error_buf[..payload_len]) };
        let str_buf = BorrowingStrBuf::new(encoded_str);

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut conn.connector,
            str_buf,
            MsgType::RodsApiReq,
            APN::Authentication as i32,
            &mut conn.msg_buf,
            &mut conn.header_buf,
        );

        let (_, _) = read_header_and_borrowing_msg::<_, XML, BorrowingStrBuf>(
            &mut conn.msg_buf,
            &mut conn.header_buf,
            &mut conn.connector,
        )?;

        Ok(signature)
    }
}
