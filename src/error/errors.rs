use base64::DecodeError;
use irods_xml;
use native_tls::HandshakeError;
use std::{convert::Infallible, net::TcpStream, num::ParseIntError, str::Utf8Error};

use crate::error::system_error::SystemError;

use crate::types::Msg;
use derive_more::Display;
use thiserror::Error;

#[derive(thiserror::Error, Debug)]
pub enum IrodsError {
    #[error("system error: [{source}]")]
    System {
        #[from]
        source: SystemError,
    },
    #[error("user input error: [{source}]")]
    UserInput {
        #[from]
        source: UserInputError,
    },
    #[error("file driver error: [{source}]")]
    FileDriver {
        #[from]
        source: FileDriverError,
    },
    #[error("catalog library error: [{source}]")]
    CatalogLibrary {
        #[from]
        source: CatalogLibraryError,
    },
    #[error("misc error: [{source}]")]
    Misc {
        #[from]
        source: MiscError,
    },
    #[error("authentication error: [{source}]")]
    Authentication {
        #[from]
        source: AuthenticationError,
    },
    #[error("rule enginge error: [{source}]")]
    RuleEngine {
        #[from]
        source: RuleEngineError,
    },
    #[error("PHP error: [{source}]")]
    PHP {
        #[from]
        source: PHPError,
    },
    #[error("NetCDF error: [{source}]")]
    NetCDF {
        #[from]
        source: NetCDFError,
    },
    #[error("SSL error: [{source}]")]
    SSL {
        #[from]
        source: SSLError,
    },
    #[error("OOCI error: [{source}]")]
    OOCI {
        #[from]
        source: OOCIError,
    },
    #[error("XML error: [{source}]")]
    XML {
        #[from]
        source: XMLError,
    },

    #[error("io error: [{source}]")]
    IO {
        #[from]
        source: std::io::Error,
    },
    #[error("unexpected response from server, expected: [{0}]")]
    UnexpectedResponse(String),

    #[error("Error: [{0}]")]
    Other(String),

    #[error("parse int error")]
    ParseInt {
        #[from]
        source: ParseIntError,
    },

    #[error("unsupported version [{0}]")]
    UnsupportedVersion(u8),

    #[error("deserialization error: [{}]", source)]
    DeError {
        #[from]
        source: irods_xml::DeError,
    },

    #[error("error: invalid utf-8 received from server: [{}]", source)]
    Utf8 {
        #[from]
        source: Utf8Error,
    },

    #[error("problem encountered in serde_json: [{}]", source)]
    SerdeJson {
        #[from]
        source: serde_json::error::Error,
    },

    #[error("problem establishing SSL credentials: [{}]", source)]
    SSLClient {
        #[from]
        source: native_tls::Error,
    },

    #[error("problem during SSL handshkae: [{0}]", source)]
    SSLHandShake {
        #[from]
        source: HandshakeError<TcpStream>,
    },

    #[error("failed converting between int types: [{source}]")]
    TryFromIntError {
        #[from]
        source: std::num::TryFromIntError,
    },

    #[error("")]
    __Infallible {
        #[source]
        source: Infallible,
    },

    #[error("failed decoding base64 from server. error: [{source}]")]
    Base64DecodeError {
        #[source]
        source: DecodeError,
    },

    #[error("error while interacting with retrieved connection: [{source}]")]
    InteractError {
        #[source]
        source: deadpool_sync::InteractError,
    },

    #[error("[{source}]")]
    SerializationOrDeserializationError {
        #[source]
        source: irods_xml::Error,
    },

    #[error("[{source}]")]
    GoAway {
        #[source]
        source: quick_xml::Error,
    },

    #[error("cached error: [{source}]")]
    CachedError {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("b64 encoding error: [{source}]")]
    EncodeBufError {
        #[source]
        source: base64::EncodeSliceError,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum UserInputError {}

#[derive(thiserror::Error, Debug)]
pub enum FileDriverError {}

#[derive(thiserror::Error, Debug)]
pub enum DirectAccessVaultError {}

#[derive(thiserror::Error, Debug)]
pub enum CatalogLibraryError {}

#[derive(thiserror::Error, Debug)]
pub enum MiscError {}

#[derive(thiserror::Error, Debug)]
pub enum AuthenticationError {}

#[derive(thiserror::Error, Debug)]
pub enum RuleEngineError {}

#[derive(thiserror::Error, Debug)]
pub enum PHPError {}

#[derive(thiserror::Error, Debug)]
pub enum NetCDFError {}

#[derive(thiserror::Error, Debug)]
pub enum SSLError {}

#[derive(thiserror::Error, Debug)]
pub enum OOCIError {}

#[derive(thiserror::Error, Debug)]
pub enum XMLError {}

/*
pub fn check_int_info(int_info: i32) -> Result<(), IrodsError> {
    match -int_info {
        i32::MIN..=0 => Ok(()),
        1_000..=299_000 => IrodsError::System {
            source: int_info.into(),
        },
        300_000..=499_000 => IrodsError::UserInput {
            source: int_info.into(),
        },
        500_000..=799_999 => IrodsError::FileDriver {
            source: int_info.into(),
        },
        800_000..=880_000 => IrodsError::CatalogLibrary {
            source: int_info.into(),
        },
        900_000..=920_000 => IrodsError::Misc {
            source: int_info.into(),
        },
        921_000..=999_000 => IrodsError::Authentication {
            source: int_info.into(),
        },
        1_000_000..=1_500_000 => IrodsError::RuleEngine {
            source: int_info.into(),
        },
        1_600_000..=1_700_000 => IrodsError::PHP {
            source: int_info.into(),
        },
        2_000_000..=2_099_000 => IrodsError::NetCDF {
            source: int_info.into(),
        },
        2_100_000..=2_199_000 => IrodsError::SSL {
            source: int_info.into(),
        },
        2_200_000..=2_299_000 => IrodsError::OOCI {
            source: int_info.into(),
        },
        2_300_000..=2_399_000 => IrodsError::XML {
            source: int_info.into(),
        },
        _ => unreachable!("All non-deprecated iRODS errors exhausted."),
    }
}
*/

impl From<quick_xml::Error> for IrodsError {
    fn from(value: quick_xml::Error) -> Self {
        Self::Other(format!("{value}"))
    }
}
