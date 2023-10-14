pub mod frame_allocator;

pub macro pf_allocator() {
    (unsafe { &mut *crate::PAGE_FRAME_ALLOCATOR.as_mut_ptr() })
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

        core::intrinsics::transmute::<usize, Self>(pf_allocator!().request_page().unwrap())
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
    pub unsafe fn to_ref_t<Ptr: Sync + Sized>(&self) -> &Ptr {
        core::intrinsics::transmute::<&Self, &Ptr>(self)
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
