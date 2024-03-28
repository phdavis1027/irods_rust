use std::io::{self, Cursor, Read, Write};

use base64::engine::GeneralPurposeConfig;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use base64::{encoded_len, engine::GeneralPurpose};
use base64::{engine, Engine};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use md5::{Digest, Md5};
use rods_prot_msg::error::errors::IrodsError;

use crate::connection::read_from_server;
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

pub struct NativeAuthenticator {
    a_ttl: u32,
    password: String,
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
        let mut msg_cursor = Cursor::new(&mut conn.msg_buf);
        let msg_len = T::rods_borrowing_ser(str_buf, msg_cursor.get_mut())?;

        let mut header_cursor = Cursor::new(&mut conn.header_buf);
        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, header_cursor.get_mut())?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        conn.connector
            .write_all(&(header_len as u32).to_be_bytes())?;
        conn.connector.write_all(&msg_cursor.get_ref()[..msg_len])?;
        conn.connector
            .write_all(&header_cursor.get_ref()[..header_len])?;

        let mut tmp_buf = [0u8; 4];
        // Receive server reply.
        conn.connector.read_exact(&mut tmp_buf)?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(read_from_server(
            header_len,
            header_cursor.get_mut(),
            &mut conn.connector,
        )?)?;

        // After this point, there should be no extent borrows of the buffers

        header_cursor.set_position(0);
        msg_cursor.set_position(0);

        let msg: BorrowingStrBuf = T::rods_borrowing_de(read_from_server(
            header.msg_len,
            msg_cursor.get_mut(),
            &mut conn.connector,
        )?)?;

        // This is fine because the Md5 state only
        // takes up a 4-length array of u32s
        let mut digest = Md5::new();
        digest.update(msg.buf.as_bytes());

        let mut pad_buf = &mut header_cursor.get_mut()[..MAX_PASSWORD_LEN];
        pad_buf.fill(0);
        for (i, c) in self.password.as_bytes().iter().enumerate() {
            pad_buf[i] = *c;
        }
        digest.update(pad_buf);

        write!(
            header_cursor,
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
            msg.buf,
            conn.account.client_user,
            conn.account.client_zone,
            STANDARD.encode(digest.finalize())
        );

        let unencoded_len = header_cursor.position() as usize;
        let payload_len = b64_engine
            .encode_slice(
                &header_cursor.get_mut()[..unencoded_len],
                msg_cursor.get_mut().as_mut_slice(),
            )
            .map_err(|e| IrodsError::Other("FIXME: This sucks".into()))?;

        let encoded_str =
            unsafe { std::str::from_utf8_unchecked(&msg_cursor.get_ref()[..payload_len]) };
        let str_buf = BorrowingStrBuf::new(encoded_str);
        let msg_len = T::rods_borrowing_ser(str_buf, header_cursor.get_mut())?;

        let header = OwningStandardHeader::new(MsgType::RodsConnect, msg_len, 0, 0, 0);
        let header_len = T::rods_owning_ser(&header, msg_cursor.get_mut())?;

        // Panics: This won't panic because the previous serialization calls
        // expnded the buffer to the correct size
        conn.connector
            .write_all(&(header_len as u32).to_be_bytes())?;
        conn.connector.write_all(&msg_cursor.get_mut()[..msg_len])?;
        conn.connector
            .write_all(&header_cursor.get_mut()[..header_len])?;

        // Receive server reply.
        header_cursor.set_position(0);
        msg_cursor.set_position(0);

        conn.connector.read_exact(tmp_buf.as_mut())?;
        let header_len = u32::from_be_bytes(tmp_buf) as usize;

        let header: OwningStandardHeader = T::rods_owning_de(read_from_server(
            header_len,
            header_cursor.get_mut(),
            &mut conn.connector,
        )?)?;

        let msg: BorrowingStrBuf = T::rods_borrowing_de(read_from_server(
            header.msg_len,
            msg_cursor.get_mut(),
            &mut conn.connector,
        )?)?;

        Ok(Vec::new())
    }
}
