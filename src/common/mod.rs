#[cfg_attr(test, derive(Debug))]
#[derive(Eq, PartialEq)]
pub enum IrodsProt {
    XML,
    Native
}

impl From<&IrodsProt> for &str {
    fn from(value: &IrodsProt) -> Self {
        match value {
            IrodsProt::Native => "0",
            IrodsProt::XML => "1",
        }
    }
}
