pub mod bin_bytes_buf;
pub mod cond_input;
pub mod cs_neg;
pub mod data_obj_inp;
pub mod header;
pub mod key_val_pair;
pub mod spec_coll;
pub mod startup_pack;
pub mod version;

extern crate serde;

use std::io::{self, Read, Write};

use quick_xml::{events::Event, Writer};
use rods_prot_msg::{error::errors::IrodsError, types::Version};

use crate::bosd::xml::BorrowingXMLSerializable;

use self::{
    header::OwningStandardHeader, startup_pack::BorrowingStartupPack, version::BorrowingVersion,
};
