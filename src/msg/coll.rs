use crate::fs::{OpenFlag, OprType};

use super::cond_input::CondInput;

pub struct CollInp {
    pub name: String,
    flags: i32,
    pub opr_type: OprType,
    pub cond_input: CondInput,
}

impl CollInp {
    pub fn builder() -> CollInpBuilder {
        CollInpBuilder {
            name: String::new(),
            flags: 0,
            opr_type: OprType::No,
            cond_input: CondInput::new(),
        }
    }
}

pub struct CollInpBuilder {
    name: String,
    flags: i32,
    opr_type: OprType,
    cond_input: CondInput,
}

impl CollInpBuilder {
    pub fn set_flag(mut self, flag: OpenFlag) -> Self {
        self.flags |= flag as i32;
        self
    }

    pub fn unset_flag(mut self, flag: OpenFlag) -> Self {
        self.flags &= !(flag as i32);
        self
    }

    pub fn build(self) -> CollInp {
        CollInp {
            name: self.name,
            flags: self.flags,
            opr_type: self.opr_type,
            cond_input: self.cond_input,
        }
    }
}
