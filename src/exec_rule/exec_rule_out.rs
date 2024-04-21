use std::io::Cursor;

use quick_xml::events::Event;

use super::ExecRuleOut;
use crate::{
    bosd::{xml::XMLDeserializable, Deserializable},
    error::errors::IrodsError,
};

impl Deserializable for ExecRuleOut {}
impl XMLDeserializable for ExecRuleOut {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            BufOne,
            BufOneLen,
            BufOneLenInner,
            BufOneBuf,
            BufOneBufInner,
            BufTwo,
            BufTwoLen,
            BufTwoLenInner,
            BufTwoBuf,
            BufTwoBufInner,
            ExitCode,
        }

        // let mut buf_one = None;
        // let mut buf_two = None;
        // let mut exit_code = None;

        let mut state = State::Tag;

        let mut reader = quick_xml::Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                (_, Event::Eof) => {
                    return Err(crate::error::errors::IrodsError::Other(
                        "Unexpected EOF".to_string(),
                    ))
                }
                state => state.0,
            }
        }
    }
}
