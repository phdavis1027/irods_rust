use std::{
    fs::File,
    io::{BufReader, Read},
    marker::PhantomData,
    net::TcpStream,
    path::PathBuf,
    time::Duration,
};

use native_tls::TlsStream;

pub type TlsStream = TlsStream<TcpStream>;

pub struct IrodsSSLSettings {
    pub hash_rounds: u32,
    pub key_size: usize,
    pub salt_size: usize,
    pub algorithm: String,
    pub cert_file: PathBuf,
    pub domain: String,
}

impl IrodsSSLSettings {
    fn from_irods_environment() -> Option<IrodsSSLSettings> {
        let env = match File::open("/etc/irods_environment.json") {
            Ok(f) => BufReader::new(f),
            Err(_) => return None,
        };

        let env: serde_json::Map<String, serde_json::value::Value> =
            match serde_json::from_reader(env).ok() {
                Some(value) => value,
                None => return None,
            };

        env.try_into().ok()
    }
}
