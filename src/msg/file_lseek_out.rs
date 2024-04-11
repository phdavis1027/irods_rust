use rods_prot_msg::error::errors::IrodsError;

use crate::bosd::{xml::XMLDeserializable, Deserializable};

#[derive(Debug)]
pub struct FileLseekOut {
    pub offset: usize,
}

impl Deserializable for FileLseekOut {}
impl XMLDeserializable for FileLseekOut {
    fn from_xml(xml: &[u8]) -> Result<Self, IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            Offset,
            OffsetInner,
        }

        let mut offset: Option<usize> = None;

        let mut state = State::Tag;

        let mut reader = quick_xml::Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, quick_xml::events::Event::Start(ref e))
                    if e.name().as_ref() == b"offset" =>
                {
                    State::Offset
                }
                (State::Offset, quick_xml::events::Event::Start(ref e))
                    if e.name().as_ref() == b"offset" =>
                {
                    State::OffsetInner
                }
                (State::OffsetInner, quick_xml::events::Event::Text(e)) => {
                    offset = Some(e.unescape()?.parse()?);
                    return Ok(Self {
                        offset: offset.unwrap(),
                    });
                }
                (_, quick_xml::events::Event::Eof) => {
                    return Err(IrodsError::Other("Unexpected EOF".to_string()));
                }
                state => state.0,
            };
        }
    }
}
