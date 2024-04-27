use std::{
    io::{Cursor, Write},
    usize,
};

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
    pub max_rows: u32,
    pub continue_index: usize,
    pub partial_start_inx: usize,
    pub flags: u32,
    pub options: CondInput,
    pub selects: Vec<IcatColumn>,
    pub conditions: Vec<(IcatColumn, IcatPredicate)>,
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
        tag_fmt!(writer, "options", "{}", self.flags);

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
        tag_fmt!(writer, "isLen", "{}", self.conditions.len());
        for (column, _) in &self.conditions {
            tag_fmt!(writer, "inx", "{}", *column as u32);
        }
        for (_, predicate) in &self.conditions {
            match predicate {
                IcatPredicate::Equals(value) => {
                    tag_fmt!(writer, "svalue", "= &apos;{}&apos;", value);
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

#[derive(Debug, Default)]
pub struct GenQueryOut {
    pub row_count: u32,
    pub attr_count: u32,
    pub continue_index: usize,
    pub total_row_count: u32,
    pub columns: Vec<(IcatColumn, Vec<String>)>,
}

impl Deserializable for GenQueryOut {}
impl XMLDeserializable for GenQueryOut {
    fn from_xml(xml: &[u8]) -> Result<Self, crate::error::errors::IrodsError>
    where
        Self: Sized,
    {
        #[derive(Debug)]
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
        let mut columns: Vec<(IcatColumn, Vec<String>)> = Vec::new();

        let mut column_inx: Option<IcatColumn> = None; // Default
        let mut column = Vec::new();

        let mut state = State::Tag;

        let mut reader = Reader::from_reader(xml);

        loop {
            state = match (state, reader.read_event()?) {
                // <GenQueryOut_PI> -> <rowCnt>
                (State::Tag, Event::Start(e)) if e.name().as_ref() == b"GenQueryOut_PI" => {
                    State::RowCnt
                }
                // <rowCnt> -> value
                (State::RowCnt, Event::Start(e)) if e.name().as_ref() == b"rowCnt" => {
                    State::RowCntInner
                }
                // <rowCnt>value</rowCnt> -> </rowCnt>
                (State::RowCntInner, Event::Text(e)) => {
                    row_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::AttrCnt
                }
                // </rowCnt> -> <attriCnt>
                (State::RowCntInner, Event::End(e)) if e.name().as_ref() == b"rowCnt" => {
                    State::AttrCnt
                }
                // <attriCnt> -> value
                (State::AttrCnt, Event::Start(e)) if e.name().as_ref() == b"attriCnt" => {
                    State::AttrCntInner
                }
                // <attriCnt>value</attriCnt> -> </attriCnt>
                (State::AttrCntInner, Event::Text(e)) => {
                    attr_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::ContinueInx
                }
                // </attriCnt> -> <continueInx>
                (State::AttrCntInner, Event::End(e)) if e.name().as_ref() == b"attriCnt" => {
                    State::ContinueInx
                }
                // <continueInx> -> value
                (State::ContinueInx, Event::Start(e)) if e.name().as_ref() == b"continueInx" => {
                    State::ContinueInxInner
                }
                // <continueInx>value</continueInx> -> </continueInx>
                (State::ContinueInxInner, Event::Text(e)) => {
                    continue_index = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::TotalRowCnt
                }
                // </continueInx> -> <totalRowCount>
                (State::ContinueInxInner, Event::End(e)) if e.name().as_ref() == b"continueInx" => {
                    State::TotalRowCnt
                }
                // <totalRowCount> -> value
                (State::TotalRowCnt, Event::Start(e)) if e.name().as_ref() == b"totalRowCount" => {
                    State::TotalRowCntInner
                }
                // <totalRowCount>value</totalRowCount> -> </totalRowCount>
                (State::TotalRowCntInner, Event::Text(e)) => {
                    total_row_count = Some(e.unescape_with(irods_unescapes)?.parse()?);
                    State::Results
                }
                // </totalRowCount> -> <SqlResult_PI>
                (State::TotalRowCntInner, Event::End(e))
                    if e.name().as_ref() == b"totalRowCount" =>
                {
                    State::Results
                }
                // <SqlResult_PI> -> <attriInx>
                (State::Results, Event::Start(e)) if e.name().as_ref() == b"SqlResult_PI" => {
                    if columns.len() >= attr_count.unwrap() as usize {
                        return Ok(Self {
                            row_count: row_count.unwrap(),
                            attr_count: attr_count.unwrap(),
                            continue_index: continue_index.unwrap(),
                            total_row_count: total_row_count.unwrap(),
                            columns,
                        });
                    }
                    State::ResultsInnerAttrInx
                }
                // <attriInx> -> value
                (State::ResultsInnerAttrInx, Event::Start(e))
                    if e.name().as_ref() == b"attriInx" =>
                {
                    State::ResultsInnerAttrInxInner
                }
                // <attriInx>value</attriInx> -> </attriInx>
                (State::ResultsInnerAttrInxInner, Event::Text(e)) => {
                    column_inx = Some(
                        e.unescape_with(irods_unescapes)?
                            .as_ref()
                            .try_into()
                            .map_err(|_| {
                                IrodsError::Other("Failed to convert column index".to_string())
                            })?,
                    );
                    State::ResultsInnerResLen
                }
                // </attriInx> -> <reslen>
                (State::ResultsInnerAttrInxInner, Event::End(e))
                    if e.name().as_ref() == b"attriInx" =>
                {
                    State::ResultsInnerResLen
                }
                // <reslen> -> value
                (State::ResultsInnerResLen, Event::Start(e)) if e.name().as_ref() == b"reslen" => {
                    State::ResultsInnerResLenInner
                }
                // <reslen>value</reslen> -> </reslen>
                (State::ResultsInnerResLenInner, Event::Text(_)) => State::ResultsInnerValue,
                // </reslen>
                (State::ResultsInnerResLenInner, Event::End(e))
                    if e.name().as_ref() == b"reslen" =>
                {
                    State::ResultsInnerValue
                }
                // <value> -> value
                (State::ResultsInnerValue, Event::Start(e)) if e.name().as_ref() == b"value" => {
                    State::ResultsInnerValueInner
                }
                // <value>value</value> -> </value>
                (State::ResultsInnerValue, Event::End(e))
                    if e.name().as_ref() == b"SqlResult_PI" =>
                {
                    columns.push((column_inx.unwrap(), std::mem::take(&mut column)));
                    State::Results
                }
                // <value>value</value> -> </value>
                (State::ResultsInnerValueInner, Event::Text(e)) => {
                    column.push(e.unescape_with(irods_unescapes)?.to_string());
                    State::ResultsInnerValue
                }
                // </value> -> <attriInx>
                (State::ResultsInnerValueInner, Event::End(e)) if e.name().as_ref() == b"value" => {
                    State::ResultsInnerValue
                }
                (_, Event::Eof) => return Err(IrodsError::Other("Unexpected EOF".to_string())),
                state => state.0,
            }
        }
    }
}
