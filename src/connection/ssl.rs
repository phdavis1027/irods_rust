use std::{fs::File, io::BufReader, net::TcpStream, path::PathBuf, time::Duration};

use native_tls::TlsStream;
use rods_prot_msg::error::errors::IrodsError;

pub type SslStream = TlsStream<TcpStream>;

#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
#[derive(Clone)]
pub struct IrodsSSLSettings {
    pub hash_rounds: u32,
    pub key_size: usize,
    pub salt_size: usize,
    pub algorithm: String,
    pub cert_file: PathBuf,
    pub domain: String,
}

impl TryFrom<serde_json::Map<String, serde_json::value::Value>> for IrodsSSLSettings {
    type Error = IrodsError;

    fn try_from(
        value: serde_json::Map<String, serde_json::value::Value>,
    ) -> Result<Self, Self::Error> {
        let key_size: usize = match value.get("irods_encryption_key_size") {
            // The key
            // exists and represents a number
            // FIXME: Is this clone really necessary?
            Some(s) => serde_json::from_value(s.clone())?,
            None => {
                return Err(IrodsError::Other(
                    "No key `irods_encryption_key_size` found in `irods_environment.json`".into(),
                ))
            }
        };

        let algorithm: String = match value.get("irods_encryption_algorithm") {
            Some(s) => serde_json::from_value(s.clone())?,
            None => {
                return Err(IrodsError::Other(
                    "no key `irods_encryption_algorithm` found in irods_environment.json".into(),
                ))
            }
        };

        let salt_size: usize = match value.get("irods_encryption_salt_size") {
            // The key
            // exists and represents a number
            // FIXME: Is this clone really necessary?
            Some(s) => serde_json::from_value(s.clone())?,
            None => {
                return Err(IrodsError::Other(
                    "no key `irods_encryption_salt_size` found in irods_environment.json".into(),
                ))
            }
        };

        let hash_rounds: u32 = match value.get("irods_encryption_hash_rounds") {
            // The key
            // exists and represents a number
            // FIXME: Is this clone really necessary?
            Some(s) => serde_json::from_value(s.clone())?,
            None => {
                return Err(IrodsError::Other(
                    "no key `irods_encryption_hash_rounds` found in irods_environment.json".into(),
                ))
            }
        };

        let cert_file: String = match value.get("irods_ca_certificate_file") {
            // The key
            // exists and represents a number
            // FIXME: Is this clone really necessary?
            Some(s) => serde_json::from_value(s.clone())?,
            None => {
                return Err(IrodsError::Other(
                    "no key `irods_ca_certificate_file` found in irods_environment.json".into(),
                ))
            }
        };
        let cert_file: PathBuf = cert_file.into();

        // FIXME: I don't think the domain normally lives in iRODS environment
        Ok(IrodsSSLSettings {
            cert_file,
            hash_rounds,
            salt_size,
            algorithm,
            key_size,
            domain: "localhost".into(),
        })
    }
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
