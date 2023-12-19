use core::marker;

use bit::BitIndex;
use numtoa::NumToA;

use crate::memory::PhysicalAddress;

pub mod frame_allocator;
pub mod table_manager;
pub mod indexer;

#[deprecated = "This is not thread-safe at all, a better method is work in process."]
/// UNSAFETY: Get a mutable reference to a static that might not be initialized(**`MaybeUnit`**).
/// Don't use if you don't know exactly what your doing, even if you do, using this will probably lead to **UB**.
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

pub use x86_64::structures::paging::page_table::PageTableEntry;
pub use x86_64::structures::paging::page_table::PageTable;

pub struct PageFrameMapper;
