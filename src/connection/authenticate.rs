use std::io::{self, Cursor, Read, Write};

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::xml::XML;
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

pub trait Authenticate<T, C>
where
    T: BorrowingSerializer + BorrowingDeserializer,
    T: OwningSerializer + OwningDeserializer,
    C: io::Read + io::Write,
{
    type Output;

    fn authenticate(&self, conn: &mut Connection<T, C>) -> Result<Self::Output, IrodsError>;
}

#[derive(Clone)]
pub struct NativeAuthenticator {
    a_ttl: u32,
    password: String,
}

impl NativeAuthenticator {
    fn create_base64_engine() -> GeneralPurpose {
        let cfg = GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true);
        GeneralPurpose::new(&base64::alphabet::STANDARD, cfg)
    }

    pub fn new(a_ttl: u32, password: String) -> Self {
        Self { a_ttl, password }
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
        let b64_engine = Self::create_base64_engine();

        // UNSAFE: Connection buffers are always initialized with
        // at least enough space for the payload
        let mut unencoded_cursor = Cursor::new(&mut conn.unencoded_buf);

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
        let payload_len = b64_engine
            .encode_slice(
                &unencoded_cursor.get_mut()[..unencoded_len], // UNSAFE: The cursor guarantees we have enough space
                conn.encoded_buf.as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks.".into()))?;

        // UNSAFE: Base64 is always valid UTF-8
        let encoded_str =
            unsafe { std::str::from_utf8_unchecked(&conn.encoded_buf[..payload_len]) };
        let str_buf = BorrowingStrBuf::new(encoded_str);

        // We're being very naughty here and serializing the msg into the
        // thing called "header" buf. This unfortunate, but I can't think
        // of a better way to do it right now that gets around the
        // borrow checker.
        send_borrowing_msg_and_header::<XML, _, _>(
            &mut conn.connector,
            str_buf,
            MsgType::RodsApiReq,
            0,
            &mut conn.msg_buf,
            &mut conn.header_buf,
        );

        let (_, challenge) = read_header_and_borrowing_msg::<_, XML, BorrowingStrBuf>(
            &mut conn.msg_buf,
            &mut conn.header_buf,
            &mut conn.connector,
        )?;

        // This is fine because the Md5 state only
        // takes up a 4-length array of u32s
        let mut digest = Md5::new();
        digest.update(challenge.buf.as_bytes());

        // Briefly repurpose the unencoded buf
        let mut pad_buf = &mut conn.unencoded_buf[..MAX_PASSWORD_LEN];

        pad_buf.fill(0);
        for (i, c) in self.password.as_bytes().iter().enumerate() {
            pad_buf[i] = *c;
        }
        digest.update(pad_buf);

        let mut unencoded_cursor = Cursor::new(&mut conn.unencoded_buf);
        write!(
            unencoded_cursor,
            r#"
        {{
            "a_ttl": {0},
            "force_password_prompt": "true",
            "next_operation": "auth_agent_auth_response",
            "request_result": "{1}",
            "scheme": "native",
            "user_name": "{2}",
            "zone_name": "{3}",
            "digest": "{4}"
        }}"#,
            self.a_ttl,
            challenge.buf,
            conn.account.client_user,
            conn.account.client_zone,
            STANDARD.encode(digest.finalize())
        );

        let unencoded_len = unencoded_cursor.position() as usize;
        let payload_len = b64_engine
            .encode_slice(
                &unencoded_cursor.get_mut()[..unencoded_len],
                conn.encoded_buf.as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks".into()))?;

        let encoded_str = unsafe { std::str::from_utf8_unchecked(&conn.encoded_buf) };
        let str_buf = BorrowingStrBuf::new(encoded_str);

        send_borrowing_msg_and_header::<XML, _, _>(
            &mut conn.connector,
            str_buf,
            MsgType::RodsApiReq,
            0,
            &mut conn.msg_buf,
            &mut conn.header_buf,
        );

        let (_, _) = read_header_and_borrowing_msg::<_, XML, BorrowingStrBuf>(
            &mut conn.msg_buf,
            &mut conn.header_buf,
            &mut conn.connector,
        )?;

        Ok(Vec::new())
    }
}
