use crate::common::{cond_input_kw::CondInputKw, icat_column::IcatColumn};

use super::cond_input::CondInput;

pub enum IcatPredicate {
    Equals(String),
}

pub struct GenQueryInp {
    max_rows: u32,
    continue_index: usize,
    partial_start_inx: usize,
    flags: u32,
    options: CondInput,
    selects: Vec<IcatColumn>,
    conditions: Vec<(IcatColumn, IcatPredicate)>,
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
