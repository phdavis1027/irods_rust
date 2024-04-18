pub mod cond_input_kw;
pub mod icat_column;

use crate::error::errors::IrodsError;

#[derive(Debug, Eq, PartialEq)]
pub enum IrodsProt {
    XML,
    Native,
}

impl From<&IrodsProt> for &str {
    fn from(value: &IrodsProt) -> Self {
        match value {
            IrodsProt::Native => "0",
            IrodsProt::XML => "1",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CsNegPolicy {
    CS_NEG_REFUSE,
    CS_NEG_REQUIRE,
    CS_NEG_DONT_CARE,
}

impl From<&CsNegPolicy> for &str {
    fn from(value: &CsNegPolicy) -> Self {
        match value {
            CsNegPolicy::CS_NEG_REFUSE => "CS_NEG_REFUSE",
            CsNegPolicy::CS_NEG_REQUIRE => "CS_NEG_REQUIRE",
            CsNegPolicy::CS_NEG_DONT_CARE => "CS_NEG_DONT_CARE",
        }
    }
}

impl TryFrom<&str> for CsNegPolicy {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CS_NEG_REFUSE" => Ok(CsNegPolicy::CS_NEG_REFUSE),
            "CS_NEG_REQUIRE" => Ok(CsNegPolicy::CS_NEG_REQUIRE),
            "CS_NEG_DONT_CARE" => Ok(CsNegPolicy::CS_NEG_DONT_CARE),
            _ => Err(IrodsError::Other("Invalid value for CsNegPolicy".into())),
        }
    }
}

#[derive(Debug)]
pub enum CsNegResult {
    CS_NEG_FAILURE,
    CS_NEG_USE_SSL,
    CS_NEG_USE_TCP,
}

impl From<&CsNegResult> for &str {
    fn from(value: &CsNegResult) -> Self {
        match value {
            CsNegResult::CS_NEG_FAILURE => "CS_NEG_FAILURE",
            CsNegResult::CS_NEG_USE_SSL => "CS_NEG_USE_SSL",
            CsNegResult::CS_NEG_USE_TCP => "CS_NEG_USE_TCP",
        }
    }
}

impl TryFrom<&str> for CsNegResult {
    type Error = IrodsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "CS_NEG_FAILURE" => Ok(CsNegResult::CS_NEG_FAILURE),
            "CS_NEG_USE_SSL" => Ok(CsNegResult::CS_NEG_USE_SSL),
            "CS_NEG_USE_TCP" => Ok(CsNegResult::CS_NEG_USE_TCP),
            _ => Err(IrodsError::Other("Invalid value for CsNegResult".into())),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ObjectType {
    UnknownObj = 0,
    DataObj = 1,
    Coll = 2,
    UnknownFile = 3,
    LocalFile = 4,
    LocalDir = 5,
    NoInput = 6,
}

pub enum APN {
    Authentication = 110000,
    DataObjOpen = 602,
    DataObjClose = 673,
    DataObjLSeek = 674,
    DataObjRead = 675,
    ObjStat = 633,
    ExecMyRule = 625,
}
