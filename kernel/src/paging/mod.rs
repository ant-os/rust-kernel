use core::marker;

use bit::BitIndex;
use numtoa::NumToA;

use crate::memory::PhysicalAddress;

pub mod frame_allocator;
pub mod table_manager;
pub mod indexer;


pub macro pf_allocator() {
    (unsafe { &mut *crate::PAGE_FRAME_ALLOCATOR.as_mut_ptr() })
}

/// Get the Global [table_manager::PageTableManager].
/// 
/// **If a Global Page Table Manager wasn't set using [table_manager::PageTableManager::make_global] this result in undefined behavior!**
pub macro pt_manager(){
    & *crate::PAGE_TABLE_MANAGER.as_ptr()
}

#[must_use]
#[cfg_attr(packed, raw)]
pub struct SafePagePtr(usize)
where
    Self: Sync + Sized;

impl SafePagePtr {
    #[inline]
    pub fn new() -> Self {
        unimplemented!();
    }

    #[inline]
    pub unsafe fn unsafe_from_addr(addr: usize) -> Self {
        core::intrinsics::assume(addr != 0);
        core::intrinsics::assume(addr != usize::MAX);

        core::intrinsics::transmute::<usize, Self>(addr)
    }

    #[inline]
    pub unsafe fn as_ptr(self) -> *const u8 {
        core::intrinsics::transmute::<Self, *const u8>(self)
    }

    #[inline]
    pub unsafe fn as_mut_ptr(self) -> *mut u8 {
        core::intrinsics::transmute::<Self, *mut u8>(self)
    }

    #[inline]
    pub unsafe fn to_ref_t<T: Sync + Sized>(&self) -> &T {
        core::intrinsics::transmute::<&Self, &T>(self)
    }

    #[inline]
    pub unsafe fn to_mut_ref_t<Ptr: Sync + Sized>(&mut self) -> &mut Ptr {
        core::intrinsics::transmute::<&mut Self, &mut Ptr>(self)
    }

    #[inline]
    pub unsafe fn unchecked_raw_transmute<T>(self) -> *mut T {
        core::intrinsics::assume(core::mem::size_of::<T>() <= crate::consts::PAGE_SIZE as usize);
        core::intrinsics::transmute_unchecked::<Self, *mut T>(self)
    }

    #[inline]
    #[must_use]
    pub fn free(&mut self) {
        pf_allocator!().free_page(self.0).unwrap();
    }
}

impl Drop for SafePagePtr {
    fn drop(&mut self) {
        self.free();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PageTableEntry(usize);

impl PageTableEntry{
    pub fn present(&self) -> bool{
        self.0.bit(1)
    }

    pub fn set_present(&mut self, value: bool){
        self.0.set_bit(0, value);
    }

    pub fn rw(&self) -> bool{
        self.0.bit(1)
    }

    pub fn set_rw(&mut self, value: bool){
        self.0.set_bit(1, value);
    }

    pub fn addr(&self) -> PhysicalAddress{
        PhysicalAddress::new(self.0 & 0x0000_FFFF_FFFF_F000)
    }

    pub fn set_addr(&mut self, value: usize){
        self.0 |= value
    }

    pub fn data(&self) -> usize{
        self.0
    }

   
}

#[repr(align(0x1000))]
pub struct PageTable{
    pub entries: [PageTableEntry; 512]
}


