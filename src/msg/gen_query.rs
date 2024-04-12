use super::cond_input::CondInput;

pub struct Selects {
    pub ii_len: usize,
    pub inx: Vec<IcatColumn>,
}

pub struct GenQueryInp {
    max_rows: u32,
    continue_index: u32,
    partial_start_inx: usize,
    options: CondInput,
}
