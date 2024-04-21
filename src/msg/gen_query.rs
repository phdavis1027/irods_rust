use std::io::{Cursor, Write};

use quick_xml::{
    events::{BytesEnd, BytesStart, Event},
    Reader, Writer,
};

use crate::{
    bosd::{
        xml::{
            irods_escapes, irods_unescapes, XMLDeserializable, XMLSerializable,
            XMLSerializableChild,
        },
        Deserializable, Serialiazable,
    },
    common::{cond_input_kw::CondInputKw, icat_column::IcatColumn},
    error::errors::IrodsError,
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
    fn to_xml(&self, sink: &mut Vec<u8>) -> Result<usize, IrodsError> {
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
                    tag_fmt!(writer, "svalue", "=&apos;{}&apos;", value);
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
    fn from_xml(xml: &[u8]) -> Result<Self, crate::error::errors::IrodsError>
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

        let mut column = IcatColumn::UserId; // Default

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
                    row_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::AttrCnt
                }
                (State::AttrCnt, Event::Start(e)) if e.name().as_ref() == b"attriCnt" => {
                    State::AttrCntInner
                }
                (State::AttrCntInner, Event::Text(e)) => {
                    attr_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::ContinueInx
                }
                (State::ContinueInx, Event::Start(e)) if e.name().as_ref() == b"continueInx" => {
                    State::ContinueInxInner
                }
                (State::ContinueInxInner, Event::Text(e)) => {
                    continue_index = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::TotalRowCnt
                }
                (State::TotalRowCnt, Event::Start(e)) if e.name().as_ref() == b"totalRowCount" => {
                    State::TotalRowCntInner
                }
                (State::TotalRowCntInner, Event::Text(e)) => {
                    total_row_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::Results
                }
                (State::Results, Event::Start(e)) if e.name().as_ref() == b"SqlResult_PI" => {
                    State::ResultsInnerAttrInx
                }
                (State::ResultsInnerAttrInx, Event::Start(e))
                    if e.name().as_ref() == b"attriInx" =>
                {
                    State::ResultsInnerAttrInxInner
                }
                (State::ResultsInnerAttrInxInner, Event::Text(e)) => {
                    column = e.unescape_with(irods_unescapes)?.as_ref().try_into()?;
                    State::ResultsInnerResLen
                }
                (State::ResultsInnerResLen, Event::Start(e)) if e.name().as_ref() == b"reslen" => {
                    State::ResultsInnerResLenInner
                }
                (State::ResultsInnerResLenInner, Event::Text(_)) => State::ResultsInnerValue,
                (State::ResultsInnerValue, Event::Start(e)) if e.name().as_ref() == b"value" => {
                    State::ResultsInnerValueInner
                }
                (State::ResultsInnerValueInner, Event::Text(e)) => {
                    let result = e.unescape_with(irods_unescapes)?.to_string();
                    results.push(SqlResult {
                        attri_inx: column,
                        value: result,
                    });
                    State::Results
                }
                (State::Results, Event::End(e)) if e.name().as_ref() == b"GenQueryOut_PI" => {
                    return Ok(Self {
                        row_count: row_count
                            .ok_or_else(|| IrodsError::Other("Missing row count".to_string()))?,
                        attr_count: attr_count.ok_or_else(|| {
                            IrodsError::Other("Missing attribute count".to_string())
                        })?,
                        continue_index: continue_index.ok_or_else(|| {
                            IrodsError::Other("Missing continue index".to_string())
                        })?,
                        total_row_count: total_row_count.ok_or_else(|| {
                            IrodsError::Other("Missing total row count".to_string())
                        })?,
                        results,
                    });
                }
                (_, Event::Eof) => return Err(IrodsError::Other("Unexpected EOF".to_string())),
                state => state.0,
            }
        }
    }
}
