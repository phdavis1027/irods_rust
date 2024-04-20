use crate::{
    bosd::{xml::XMLDeserializable, Deserializable},
    error::errors::IrodsError,
};

use super::RuleOutput;

impl<T> Deserializable for RuleOutput<T> where T: Deserializable {}
impl<T> XMLDeserializable for RuleOutput<T>
where
    T: Deserializable,
{
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            ParamLen,
            ParamLenInner,
            OprType,
            OprTypeInner,
        }

        unimplemented!()
    }
}
