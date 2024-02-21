//! RSDP

use core::{
    convert::Infallible,
    ffi::{c_char, c_uchar},
    fmt::Debug,
    mem::ManuallyDrop,
    ops::Deref,
};

#[repr(C, packed)]
#[derive(Debug)]
pub struct RsdpBase {
    pub signature: CCharArray<8>,
    pub checksum: u8,
    pub oemid: CCharArray<6>,
    pub revision: u8,
    pub addr: u32,
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct RsdpExtended {
    pub signature: CCharArray<8>,
    pub checksum: u8,
    pub oemid: CCharArray<6>,
    pub revision: u8,

    #[deprecated(since = "2.0")]
    pub addr: u32,

    // Extended
    pub lenght: u32,
    pub attr_ext: u64,
    pub checksum_ext: u8,
    pub reserved: [u8; 3],
}

#[repr(C)]
/// An Root System Descriptor Pointer that can be v2 or v1.
///
/// The Rsdp is NEVER dropped by the kernel as it belongs to the Firmware.
pub union Rsdp {
    pub base: ManuallyDrop<RsdpBase>,
    pub xsdp: ManuallyDrop<RsdpExtended>,
}

impl Rsdp {
    pub fn is_extended(&self) -> bool {
        match unsafe { self.base.revision } {
            0 | 1 => false,
            _ => true,
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CCharArray<const LEN: usize>(pub [c_uchar; LEN]);

impl<const LEN: usize> Debug for CCharArray<LEN> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{:?}", rust_chars::<LEN>(&self.0)))
    }
}

pub const fn rust_chars<const LEN: usize>(c_chars: &[c_uchar; LEN]) -> [char; LEN] {
    let mut result = ['\0'; LEN];

    for i in 0..LEN {
        result[i] = (c_chars[i] as u8).into();
    }

    result
}
