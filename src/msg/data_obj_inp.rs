use std::borrow::Cow;

use rods_prot_msg::types::OprType;

use crate::fs::{CreateMode, OpenFlags};

pub struct BorrowingDataObjInp<'s> {
    pub path: Cow<'s, str>,
    pub create_mode: CreateMode,
    pub open_flags: OpenFlags,
    pub opr_type: OprType,
    pub offset: usize,
    pub data_size: usize,
    pub num_threads: u16,
}
