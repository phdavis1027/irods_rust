use std::io::{Cursor, Write};

use md5::Digest;
use rand::RngCore;

use crate::{
    bosd::ProtocolEncoding,
    connection::{Connection, MAX_PASSWORD_LEN},
    error::errors::IrodsError,
};

pub const SCRAMBLE_PADDING: &str = "1gCBizHWbwIYyWLoysGzTe6SyzqFKMniZX05faZHWAwQKXf6Fs";
pub const PREFIX: &str = "A.ObfV2";
pub const DEFAULT_PASSWORD_KEY: &str = "a9_3fker";

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // Leaves the password in the bytes buf
    fn scramble(&mut self, password: &str) -> Result<usize, IrodsError> {
        if password.is_empty() {
            return Err(IrodsError::Other("Password is empty".into()));
        } else if password.len() > MAX_PASSWORD_LEN {
            return Err(IrodsError::Other("Password is too long".into()));
        }

        let mut len_copy = MAX_PASSWORD_LEN - 10 - password.len();

        if len_copy > 15 {
            if len_copy > SCRAMBLE_PADDING.len() {
                len_copy = SCRAMBLE_PADDING.len();
            }
        }

        let mut cursor = Cursor::new(&mut self.resources.msg_buf);
        cursor.write_all(password.as_bytes())?;
        cursor.write_all(SCRAMBLE_PADDING.as_bytes()[..len_copy].as_ref())?;

        let unencrypted_len = cursor.position() as usize;

        // We skip some safety checks from the Go client because
        // they are obviously unnecessary

        let mut rand = rand::thread_rng();

        let mut cursor = Cursor::new(&mut self.resources.error_buf[1..]);
        rand.fill_bytes(&mut cursor.get_mut()[..1]);
        cursor.write_all(PREFIX.as_bytes())?;
        cursor.write_all(&mut self.resources.msg_buf[..unencrypted_len])?;

        let to_scramble_len = cursor.position() as usize;

        // Key buf
        let mut cursor = Cursor::new(&mut self.resources.header_buf);
        cursor.write_all(self.account.password.as_bytes())?;
        cursor.write_all(self.signature.as_ref())?;
        let nzeros: i32 = 100 - (self.account.password.bytes().len() + self.signature.len()) as i32;
        if nzeros > 0 {
            let zeros = &mut self.resources.msg_buf[..nzeros as usize];
            zeros.fill(0);
            cursor.write_all(zeros)?;
        }

        let key_buf_len = cursor.position() as usize;

        let mut digest = md5::Md5::new();
        digest.update(&mut self.resources.header_buf[..key_buf_len]);
        digest.finalize_into((&mut self.resources.header_buf[..]).into());

        faster_hex::hex_encode(
            &mut self.resources.header_buf[..digest.output_size()],
            &mut self.resources.msg_buf[..digest.output_size() * 2],
        );
    }
}
