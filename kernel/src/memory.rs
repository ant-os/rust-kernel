//! Common Memory Structures, e.g [VirtualAddress].

use numtoa::NumToA as _;
use x86_64::{VirtAddr, structures::paging::PageTable};

/// Specific table to be used, needed on some architectures
//TODO: Use this throughout the code
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TableKind {
    /// Userspace page table
    User,
    /// Kernel page table
    Kernel,
}

pub(crate) const PHYSICAL_MEMORY_OFFSET: usize = 0xffffffff80000000;
pub(crate) const PHYSICAL_BOOTLOADER_MEMORY_OFFSET: u64 = 0x00;

pub(crate) unsafe fn active_level_4_table(physical_memory_offset: VirtAddr)
    -> &'static mut PageTable
{
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = VirtAddr::new_unsafe(phys.as_u64());
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
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

    pub fn to_virtual(&self) ->  VirtualAddress{
        VirtualAddress::new(PHYSICAL_MEMORY_OFFSET + (self.data() >> 12))
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

    pub unsafe fn as_ptr(&self) -> *mut u8{
        core::mem::transmute::<_, *mut u8>(self.0)
    } 
}

impl alloc::fmt::Debug for VirtualAddress{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("VirtualAddress").field_with(|v|v.write_fmt(format_args!("{:#x}",&self.0))).finish()
    }
}


#[derive(Clone, Copy, Debug)]
pub struct MemoryArea {
    pub base: VirtualAddress,
    pub size: usize,
}

impl MemoryArea{
    pub const fn new(base: usize, size: usize) -> Self{
        Self { base: VirtualAddress::new(base), size }
    }
}

