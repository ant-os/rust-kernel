//! Root System Descriptor Pointer
//! =================================
//!
//! Source: <https://wiki.osdev.org/RSDT>

use crate::rsdp::CCharArray;

#[repr(C)]
pub struct SDTHeader {
    signature: CCharArray<4>,
    length: u32,
    revision: u8,
    checksum: u8,
    oemid: CCharArray<6>,
    oem_table_id: CCharArray<8>,
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}
