use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Reader, Writer,
};

use crate::{
    bosd::{
        xml::{XMLDeserializable, XMLSerializable, XMLSerializableChild},
        Deserializable, Serialiazable,
    },
    common::{cond_input_kw::CondInputKw, icat_column::IcatColumn},
    tag_fmt,
};

use super::cond_input::CondInput;

#[derive(Debug)]
pub enum IcatPredicate {
    Equals(String),
}

#[derive(Debug)]
pub struct GenQueryInp {
    max_rows: u32,
    continue_index: usize,
    partial_start_inx: usize,
    flags: u32,
    options: CondInput,
    selects: Vec<IcatColumn>,
    conditions: Vec<(IcatColumn, IcatPredicate)>,
}

impl Serialiazable for GenQueryInp {}
impl XMLSerializable for GenQueryInp {
    fn to_xml(
        &self,
        sink: &mut Vec<u8>,
    ) -> Result<usize, rods_prot_msg::error::errors::IrodsError> {
        let mut cursor = Cursor::new(sink);
        let mut writer = Writer::new(&mut cursor);

        writer.write_event(Event::Start(BytesStart::new("GenQueryInp_PI")))?;

        tag_fmt!(writer, "maxRows", "{}", self.max_rows);
        tag_fmt!(writer, "continueInx", "{}", self.continue_index);
        tag_fmt!(writer, "partialStartIndex", "{}", self.partial_start_inx);
        tag_fmt!(writer, "flags", "{}", self.flags);

        self.options.to_nested_xml(&mut writer)?;

        writer.write_event(Event::Start(BytesStart::new("InxIvalPair_PI")))?;
        tag_fmt!(writer, "iiLen", "{}", self.selects.len());
        for column in &self.selects {
            tag_fmt!(writer, "inx", "{}", *column as u32);
        }
        for _ in 0..self.selects.len() {
            tag_fmt!(writer, "ivalue", "{}", "1");
        }
        writer.write_event(Event::End(BytesEnd::new("InxIvalPair_PI")))?;

        writer.write_event(Event::Start(BytesStart::new("InxValPair_PI")))?;
        tag_fmt!(writer, "iiLen", "{}", self.conditions.len());
        for (column, _) in &self.conditions {
            tag_fmt!(writer, "inx", "{}", *column as u32);
        }
        for (_, predicate) in &self.conditions {
            match predicate {
                IcatPredicate::Equals(value) => {
                    tag_fmt!(writer, "svalue", "='{}'", value);
                }
            }
        }
        writer.write_event(Event::End(BytesEnd::new("InxValPair_PI")))?;

        writer.write_event(Event::End(BytesEnd::new("GenQueryInp_PI")))?;

        Ok(cursor.position() as usize)
    }
}

impl Default for GenQueryInp {
    fn default() -> Self {
        Self {
            max_rows: 500,
            continue_index: 0,
            partial_start_inx: 0,
            flags: 0,
            options: CondInput::new(),
            selects: Vec::new(),
            conditions: Vec::new(),
        }
    }
}

impl GenQueryInp {
    fn builder() -> QueryBuilder {
        QueryBuilder::new()
    }
}

pub struct QueryBuilder {
    query: GenQueryInp,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            query: GenQueryInp::default(),
        }
    }

    pub fn max_rows(mut self, max_rows: u32) -> Self {
        self.query.max_rows = max_rows;
        self
    }

    pub fn continue_index(mut self, continue_index: usize) -> Self {
        self.query.continue_index = continue_index;
        self
    }

    pub fn partial_start_inx(mut self, partial_start_inx: usize) -> Self {
        self.query.partial_start_inx = partial_start_inx;
        self
    }

    pub fn flags(mut self, flags: u32) -> Self {
        self.query.flags = flags;
        self
    }

    pub fn kw(mut self, key: CondInputKw, value: String) -> Self {
        self.query.options.add_kw(key, value);
        self
    }

    pub fn select(mut self, column: IcatColumn) -> Self {
        self.query.selects.push(column);
        self
    }

    pub fn condition(mut self, column: IcatColumn, predicate: IcatPredicate) -> Self {
        self.query.conditions.push((column, predicate));
        self
    }

    pub fn build(self) -> GenQueryInp {
        self.query
    }
}

#[derive(Debug)]
pub struct SqlResult {
    pub attri_inx: IcatColumn,
    pub res_len: u32,
    pub value: String,
}

#[derive(Debug)]
pub struct GenQueryOut {
    pub row_count: u32,
    pub attr_count: u32,
    pub continue_index: usize,
    pub total_row_count: u32,
    pub results: Vec<SqlResult>,
}

impl Deserializable for GenQueryOut {}
impl XMLDeserializable for GenQueryOut {
    fn from_xml(xml: &[u8]) -> Result<Self, rods_prot_msg::error::errors::IrodsError>
    where
        Self: Sized,
    {
        #[repr(u8)]
        enum State {
            Tag,
            RowCnt,
            RowCntInner,
            AttrCnt,
            AttrCntInner,
            ContinueInx,
            ContinueInxInner,
            TotalRowCnt,
            TotalRowCntInner,
            Results,
            ResultsInnerAttrInx,
            ResultsInnerAttrInxInner,
            ResultsInnerResLen,
            ResultsInnerResLenInner,
            ResultsInnerValue,
            ResultsInnerValueInner,
        }

        let mut row_count: Option<u32> = None;
        let mut attr_count: Option<u32> = None;
        let mut continue_index: Option<usize> = None;
        let mut total_row_count: Option<u32> = None;
        let mut results: Vec<SqlResult> = Vec::new();

        let mut state = State::Tag;

        let mut reader = Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"GenQueryOut_PI" => {
                    State::RowCnt
                }
                (State::RowCnt, Event::Start(e)) if e.name().as_ref() == b"rowCnt" => {
                    State::RowCntInner
                }
                (State::RowCntInner, Event::Text(e)) => {
                    row_count = Some(e.unescape()?.parse()?);
                    State::AttrCnt
                }
                (State::AttrCnt, Event::Start(e)) if e.name().as_ref() == b"attriCnt" => {
                    State::AttrCntInner
                }
                (State::AttrCntInner, Event::Text(e)) => {
                    attr_count = Some(e.unescape()?.parse()?);
                    State::ContinueInx
                }
                _ => break,
            }
        }

        todo!()
    }
}
