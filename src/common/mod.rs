use rods_prot_msg::error::errors::IrodsError;

#[cfg_attr(test, derive(Debug))]
#[derive(Eq, PartialEq)]
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

#[cfg_attr(test, derive(Debug))]
#[derive(Eq, PartialEq)]
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
