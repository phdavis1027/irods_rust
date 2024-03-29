pub mod bin_bytes_buf;
pub mod header;
pub mod startup_pack;
pub mod version;

use std::io::{self, Read, Write};

use quick_xml::{events::Event, Writer};
use rods_prot_msg::{error::errors::IrodsError, types::Version};

use crate::bosd::xml::BorrowingXMLSerializable;

use self::{
    header::OwningStandardHeader, startup_pack::BorrowingStartupPack, version::BorrowingVersion,
};
