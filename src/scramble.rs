use md5::{digest::core_api::CoreWrapper, Digest, Md5, Md5Core};
use rand::RngCore;
use std::{
    io::{Cursor, Write},
    usize,
};

use crate::{
    bosd::ProtocolEncoding,
    connection::{Connection, MAX_PASSWORD_LEN},
    error::errors::IrodsError,
};

pub const SCRAMBLE_PADDING: &[u8; 50] = b"1gCBizHWbwIYyWLoysGzTe6SyzqFKMniZX05faZHWAwQKXf6Fs";
pub const PREFIX: &str = "A.ObfV2";
pub const DEFAULT_PASSWORD_KEY: &str = "a9_3fker";
pub const WHEEL: &[u8; 77] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!\"#$%&\\()*+,-./";

impl<T, C> Connection<T, C>
where
    T: ProtocolEncoding,
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    // Leaves the password in the bytes buf
    pub fn scramble(&mut self, password: &str) -> Result<usize, IrodsError> {
        if password.is_empty() {
            return Err(IrodsError::Other("Password is empty".into()));
        } else if password.len() > MAX_PASSWORD_LEN {
            return Err(IrodsError::Other("Password is too long".into()));
        }

        // If password is less 25 bytes, we need to pad it

        let mut cursor = Cursor::new(&mut self.resources.msg_buf);
        cursor.write_all(password.as_bytes())?;

        let mut len_copy = MAX_PASSWORD_LEN - 10 - password.len();
        if len_copy > 15 {
            if len_copy > SCRAMBLE_PADDING.len() {
                len_copy = SCRAMBLE_PADDING.len();
            }
            cursor.write_all(SCRAMBLE_PADDING.get(..len_copy).unwrap().as_ref())?;
        }

        let unencrypted_len = cursor.position() as usize;
        println!(
            "Unencrypted password and prefix: {:?}",
            std::str::from_utf8(&self.resources.msg_buf[..unencrypted_len]).unwrap()
        );

        // We skip some safety checks from the Go client because
        // they are obviously unnecessary in this context

        let mut rand = rand::thread_rng();

        // toScramble -> error_buf
        let mut cursor = Cursor::new(&mut self.resources.error_buf[1..]);
        rand.fill_bytes(&mut cursor.get_mut()[..1]);
        cursor.write_all(PREFIX.as_bytes().get(1..).unwrap())?;
        cursor.write_all(&mut self.resources.msg_buf[..unencrypted_len + 1])?;

        let to_scramble_len = cursor.position() as usize;

        println!(
            "to_Scramble: {:?}",
            std::str::from_utf8(&self.resources.error_buf[..to_scramble_len]).unwrap()
        );

        // Key buf -> header_buf
        let mut cursor = Cursor::new(&mut self.resources.header_buf);
        cursor.write_all(self.account.password.as_bytes())?;

        println!(
            "Signature: {:?}",
            std::str::from_utf8(self.signature.as_ref()).unwrap()
        );
        cursor.write_all(self.signature.as_ref())?;
        // let nzeros: i32 = 100 - (self.account.password.bytes().len() + self.signature.len()) as i32;
        // if nzeros > 0 {
        //     let zeros = &mut self.resources.msg_buf[..nzeros as usize];
        //     zeros.fill(0);
        //     cursor.write_all(zeros)?;
        // }

        let key_buf_len = cursor.position() as usize;

        println!(
            "Key buf: {:?}",
            std::str::from_utf8(&self.resources.header_buf[..key_buf_len]).unwrap()
        );

        let digest_len = CoreWrapper::<Md5Core>::output_size();

        let mut digest = Md5::new();
        digest.update(&mut self.resources.header_buf[..key_buf_len]);
        digest.finalize_into((&mut self.resources.msg_buf[..digest_len]).into());

        if self.resources.msg_buf.len() < digest_len * 2 {
            self.resources.msg_buf.resize(digest_len * 2, 0);
        }

        let ring_encoder_buf = {
            let hashed_key = faster_hex::hex_encode(
                &mut self.resources.msg_buf[..digest_len],
                &mut self.resources.bytes_buf[..digest_len * 2],
            )
            .map_err(|_| IrodsError::Other("Failed to hex encode".into()))?;

            let mut ring_encoder_buf = [0u8; 64];

            // First
            let mut digest = Md5::new();
            digest.update(hashed_key.as_bytes());
            digest.finalize_into((&mut ring_encoder_buf[..digest_len]).into());

            // Second
            let mut digest = Md5::new();
            digest.update(&ring_encoder_buf[..16]);
            digest.finalize_into((&mut ring_encoder_buf[16..16 + digest_len]).into());

            let mut digest = Md5::new();
            digest.update(&ring_encoder_buf[..32]);
            digest.finalize_into((&mut ring_encoder_buf[32..32 + digest_len]).into());

            let mut digest = Md5::new();
            digest.update(&ring_encoder_buf[..32]);
            digest.finalize_into((&mut ring_encoder_buf[48..48 + digest_len]).into());

            ring_encoder_buf
        };

        println!("Ring encoder buf: {:?}", ring_encoder_buf);

        let output_slice = &mut self.resources.bytes_buf[PREFIX.len()..];

        let mut chain = 0;

        for (i, c) in self.resources.error_buf[..to_scramble_len]
            .iter()
            .enumerate()
        {
            let mut found_in_wheel = false;
            let k = ring_encoder_buf[i % 61] as usize;
            for (j, d) in WHEEL.iter().enumerate() {
                if *d == *c {
                    let index = (j + k + chain as usize) % WHEEL.len();

                    output_slice[i] = WHEEL[index];

                    chain = WHEEL[index] & 0xff;
                    found_in_wheel = true;
                    break;
                }
            }

            if !found_in_wheel {
                output_slice[i] = *c;
            }
        }

        let _ = &mut self.resources.bytes_buf[..PREFIX.len()].copy_from_slice(PREFIX.as_bytes());

        Ok(PREFIX.len() + to_scramble_len)
    }
}
