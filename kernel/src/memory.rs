//! Common Memory Structures, e.g [VirtualAddress].

use numtoa::NumToA as _;

/// Specific table to be used, needed on some architectures
//TODO: Use this throughout the code
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TableKind {
    /// Userspace page table
    User,
    /// Kernel page table
    Kernel,
}



/// Physical memory address
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PhysicalAddress(usize);

impl PhysicalAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn data(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }

    pub fn as_str<'a>(&self) -> &'a str{
        self.0.numtoa_str(16, unsafe { &mut crate::GENERIC_STATIC_BUFFER })
    }
}

/// Virtual memory address
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VirtualAddress(usize);

impl VirtualAddress {
    #[inline(always)]
    pub const fn new(address: usize) -> Self {
        Self(address)
    }

    #[inline(always)]
    pub fn data(&self) -> usize {
        self.0
    }

    #[inline(always)]
    pub fn add(self, offset: usize) -> Self {
        Self(self.0 + offset)
    }

    #[inline(always)]
    pub fn kind(&self) -> TableKind {
        if (self.0 as isize) < 0 {
            TableKind::Kernel
        } else {
            TableKind::User
        }
    }

    pub fn as_str<'a>(&self) -> &'a str{
        self.0.numtoa_str(16, unsafe { &mut crate::GENERIC_STATIC_BUFFER })
    }
}


#[derive(Clone, Copy, Debug)]
pub struct MemoryArea {
    pub base: PhysicalAddress,
    pub size: usize,
}
