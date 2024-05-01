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
    }
}
