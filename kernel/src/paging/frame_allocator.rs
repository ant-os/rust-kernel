use core::{ptr::NonNull, sync::atomic::AtomicPtr};

use crate::{
    consts::{INVALID_PAGE_STATE, PAGE_SIZE},
    debug, endl, extra_features, memory::{MemoryArea, PhysicalAddress},
};

use alloc::sync::Arc;
use limine::{MemmapEntry, MemmapResponse, MemoryMapEntryType, NonNullPtr};
use x86_64::{structures::paging::{Size4KiB, PhysFrame}, PhysAddr};

pub type PageBitmap = bitmap::Bitmap<PtrWrapper<[usize]>, bitmap::OneBit>;

pub struct PageFrameAllocator {
    pub bitmap: &'static mut PageBitmap,
    used_memory: usize,
    free_memory: usize,
    reserved_memory: usize,
    total_memory: usize,
    _initialized: bool,
    _bitmap_index: usize,
}

macro decl_multi_page_fn([$_meta:vis] $name:ident => $target_name:ident (...)){
    $_meta fn $name (&mut self, addr: usize, num: usize) -> Result<(), Error>{
        for i in 0..num{
            self.$target_name(addr + (i * crate::consts::PAGE_SIZE as usize))?;
        }

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    UninitializedAllocator,

    OutOfMemory,
    AlreadyDone,

    CorruptedBitmap,
    OutOfBitmapBounds,

    InvalidPointer,
    InvalidAlignment,

    Continue,
}

#[derive(core::marker::ConstParamTy, PartialEq, Eq)]
pub enum PageState {
    Reserved,
    Free,
    Used,
}

impl PageFrameAllocator {
    pub unsafe fn from_response(resp: &MemmapResponse) -> PageFrameAllocator {
        if core::intrinsics::unlikely((resp).entry_count == 0) {
            unimplemented!()
        }

        let mut largest_free_segment = None;
        let mut largest_free_segment_size = 0;
        let mut total_memory = 0;

        for entry in (resp).memmap().iter() {
            if entry.typ == MemoryMapEntryType::Usable && entry.len > largest_free_segment_size {
                largest_free_segment = Some(entry);
                largest_free_segment_size = entry.len;
            }

            total_memory += entry.len;
        }


        let mut bitmap = unsafe {
            &mut *Self::place_bitmap_in_segment(
                largest_free_segment,
                (total_memory / crate::consts::PAGE_SIZE) as usize,
            )
            .as_mut_ptr()
        };

        bitmap.set(10, 1);

        let mut _self = PageFrameAllocator {
            bitmap: bitmap,
            used_memory: 0x0,
            free_memory: total_memory as usize,
            reserved_memory: 0x0,
            total_memory: total_memory as usize,
            _initialized: false,
            _bitmap_index: 0,
        };

        _self.initialize(resp, largest_free_segment.unwrap());

        _self
    }

    pub fn get_free(&mut self) -> usize {
        self.free_memory
    }

    pub fn get_used(&mut self) -> usize {
        self.used_memory
    }

    pub fn get_reserved(&mut self) -> usize {
        self.reserved_memory
    }

    pub fn get_total(&mut self) -> usize {
        self.total_memory
    }

    pub fn place_bitmap_in_segment(
        segment: Option<&NonNullPtr<MemmapEntry>>,
        pages: usize,
    ) -> &mut core::mem::MaybeUninit<PageBitmap> {
        if core::intrinsics::unlikely(segment.is_none()) {
            unimplemented!()
        }

        crate::decl_uninit! {
            _BITMAP => PageBitmap
        };

        if let Some(_seg) = segment {
            crate::assign_uninit! { _BITMAP (PageBitmap) <= unsafe {
                PageBitmap::from_storage(
                    pages + 1,
                    (),
                    PtrWrapper::<[usize]>::from_raw(core::slice::from_raw_parts_mut(
                        _seg.base as *mut usize,
                        pages + 1,
                    )),
                )
                .unwrap()
                }
            }

            return unsafe { &mut _BITMAP };
        }
        unreachable!();
    }

    pub fn initialize(
        &mut self,
        memmap: &MemmapResponse,
        bitmap_segment: &MemmapEntry,
    ) -> Option<&mut Self> {
        self.reserve_pages(0x0, self.total_memory / (PAGE_SIZE + 1) as usize);

        for entry in memmap.memmap() {
            if entry.typ == MemoryMapEntryType::Usable {
                self.unreserve_pages(
                    entry.base as usize,
                    entry.len as usize / (PAGE_SIZE as usize) + 1 as usize,
                );
            }
        }

        self.reserve_pages(0x0, 0x100);

        self.lock_pages(
            bitmap_segment.base as usize,
            (bitmap_segment.len / PAGE_SIZE) as usize + 1 as usize,
        );

        self.lock_pages(
            crate::KERNEL_BASE.physical_base as usize,
            (crate::KERNEL_FILE.length / PAGE_SIZE + 1) as usize,
        );

        for fb in crate::FRAMEBUFFERS.iter(){
            if let Some(ptr) = fb.address.as_ptr(){
                self.lock_pages(ptr.addr(), (fb.height * fb.width / PAGE_SIZE) as usize + 1);
            }
        }

        self._initialized = true;

        Some(self)
    }

    fn mark_page_as<const _state: super::frame_allocator::PageState>(
        &mut self,
        index: usize,
    ) -> Result<(), Error> {
        return match _state {
            PageState::Reserved => {
                self.bitmap.set(index, 1);
                self.reserved_memory += PAGE_SIZE as usize;
                self.free_memory -= PAGE_SIZE as usize;

                Ok(())
            }
            PageState::Free => {
                self.bitmap.set(index, 0);
                self.free_memory += PAGE_SIZE as usize;
                self.used_memory -= PAGE_SIZE as usize;

                Ok(())
            }
            PageState::Used => {
                self.bitmap.set(index, 1);
                self.used_memory += PAGE_SIZE as usize;
                self.free_memory -= PAGE_SIZE as usize;

                Ok(())
            }
            _ => unreachable!(),
        };
    }

    fn disable_page_mark<const _state: super::frame_allocator::PageState>(
        &mut self,
        index: usize,
    ) -> Result<(), Error> {
        return match _state {
            PageState::Reserved => {
                self.bitmap.set(index, 0);
                self.reserved_memory -= PAGE_SIZE as usize;
                self.free_memory += PAGE_SIZE as usize;

                Ok(())
            }
            PageState::Free => unimplemented!(),
            PageState::Used => {
                self.bitmap.set(index, 0);
                self.used_memory -= PAGE_SIZE as usize;
                self.free_memory += PAGE_SIZE as usize;

                Ok(())
            }
            _ => unreachable!(),
        };
    }

    pub fn is_used_or_reserved(&mut self, addr: usize) -> bool {
        let index = addr / PAGE_SIZE as usize;

        return match self.bitmap.get(index).unwrap_or(INVALID_PAGE_STATE) {
            0 => false,
            1 => true,
            _ => unimplemented!(),
        };
    }

    pub fn free_page(&mut self, addr: usize) -> Result<(), Error> {
        let index: usize = (addr / crate::consts::PAGE_SIZE as usize);

        let state = self.bitmap.get(index).unwrap_or(INVALID_PAGE_STATE);

        return match state {
            0 => Err(Error::AlreadyDone),
            1 => {
                /*self.bitmap.set(index, 1);
                self.free_memory += crate::consts::PAGE_SIZE as usize;
                self.used_memory -= crate::consts::PAGE_SIZE as usize;*/

                self.mark_page_as::<{ PageState::Used }>(index);

                if self._bitmap_index > index {
                    self._bitmap_index = index;
                } else {
                    return Ok(());
                }

                Ok(())
            }
            _ => Err(Error::OutOfBitmapBounds),
        };

        unreachable!();
    }

    decl_multi_page_fn! { [pub] free_pages => free_page (...) }

    pub fn lock_page(&mut self, addr: usize) -> Result<(), Error> {
        let index: usize = addr / crate::consts::PAGE_SIZE as usize;
        let state = self.bitmap.get(addr).unwrap_or(INVALID_PAGE_STATE);

        return match state {
            0 => self.mark_page_as::<{ PageState::Used }>(index),
            1 => Err(Error::AlreadyDone),
            _ => Err(Error::OutOfBitmapBounds),
        };

        unreachable!();
    }

    decl_multi_page_fn! { [pub] lock_pages => lock_page (...) }

    pub fn request_page(&mut self) -> Result<usize, Error> {
        extra_features! {
            for (_, self._bitmap_index < self.bitmap.len() * 8, self._bitmap_index += 1){

                let state = self.bitmap.get(self._bitmap_index).unwrap_or(INVALID_PAGE_STATE);

                let matched_state = match state{
                    1 => {Err(Error::Continue)}
                    0 => {
                        self.mark_page_as::<{ PageState::Used }>(self._bitmap_index)?;

                        return Ok(self._bitmap_index * PAGE_SIZE as usize);
                    },
                    _ => Err(Error::OutOfBitmapBounds)
                };

                if matched_state != Err(Error::Continue) && matched_state.is_err(){
                    return matched_state;
                }else if matched_state.is_ok(){
                    return matched_state;
                }
            }
        }

        Err(Error::OutOfMemory)
    }

    #[must_use]
    pub fn request_memory_area(&mut self, size: usize) -> Result<MemoryArea, Error>{
        let mut pages_left = (size / crate::consts::PAGE_SIZE as usize) + 1;
        let mut base: usize = 0;

        extra_features! {
            for (_, self._bitmap_index < self.bitmap.len() * 8 && pages_left > 0, self._bitmap_index += 1){

                let state = self.bitmap.get(self._bitmap_index).unwrap_or(INVALID_PAGE_STATE);

                match state{
                    1 => {
                        pages_left = size / crate::consts::PAGE_SIZE as usize;
                        base = 0;
                    },
                    0 => {
                        self.mark_page_as::<{ PageState::Used }>(self._bitmap_index)?;

                        if base == 0{
                            base = self._bitmap_index * PAGE_SIZE as usize;
                        }
                        pages_left -= 1;
                    },
                    _ => return Err(Error::OutOfBitmapBounds)
                };
            }
        }

        if base != 0{
            Ok(MemoryArea::new(PhysicalAddress::new(base).to_virtual().data(), size))
        }else{
            Err(Error::OutOfMemory)
        }
    }

    fn reserve_page(&mut self, addr: usize) -> Result<(), Error> {
        let index: usize = (addr / crate::consts::PAGE_SIZE as usize);
        let state = self.bitmap.get(addr).unwrap_or(INVALID_PAGE_STATE);

        return match state {
            0 => self.mark_page_as::<{ PageState::Reserved }>(index),
            1 => Err(Error::AlreadyDone),
            _ => Err(Error::OutOfBitmapBounds),
        };
    }

    decl_multi_page_fn! { [pub(self)] reserve_pages => reserve_page (...) }

    fn unreserve_page(&mut self, addr: usize) -> Result<(), Error> {
        let index: usize = (addr / crate::consts::PAGE_SIZE as usize);
        let state = self.bitmap.get(addr).unwrap_or(INVALID_PAGE_STATE);

        return match state {
            1 => self.disable_page_mark::<{ PageState::Reserved }>(index),
            0 => Err(Error::AlreadyDone),
            _ => Err(Error::OutOfBitmapBounds),
        };
    }

    decl_multi_page_fn! { [pub(self)] unreserve_pages => unreserve_page (...) }

    pub fn is_initialized(&self) -> bool {
        return self._initialized;
    }

    /*pub fn free_page(&mut self, addr: usize) -> Result<(), Error>{
        if !self.is_initialized(){
            return Err(Error::UninitializedAllocator)
        }else{
            return _internal_free_page(self, addr);
        }

        unreachable!()
    }*/

    /*
       This is kinda messy but we'll use it anyways...
    */
    //crate::make_wrapper! { ( free_page(addr:usize) ==> _internal_free_page ) for Self[<(), Error>] @ uninit_err = Error::UninitializedAllocator }
    // crate::make_wrapper! { ( free_pages(addr:usize, num:usize) ==> _internal_free_pages ) for Self[<(), Error>] @ uninit_err = Error::UninitializedAllocator  }

    /// Safe version of `request_page`.
    pub fn request_safe_page<'a>(&mut self) -> Result<super::SafePagePtr, Error> {
        Ok(unsafe { super::SafePagePtr::unsafe_from_addr(self.request_page()?) })
    }
}

pub struct PtrWrapper<T: ?Sized>{
    pub(self) inner: NonNull<T>
}

unsafe impl<T: ?Sized> Sync for PtrWrapper<T> {}
unsafe impl<T: ?Sized> Send for PtrWrapper<T> {}


impl<T: ?Sized> PtrWrapper<T> {
    pub unsafe fn from_raw(_val: &mut T) -> PtrWrapper<T> {
        Self { inner: NonNull::new(_val).unwrap() }
    }
}

impl bitmap::Storage for PtrWrapper<[usize]> {
    fn as_ref(&self) -> &[usize] {
        unsafe { self.inner.as_ref() }
    }

    fn as_mut(&mut self) -> &mut [usize] {
        unsafe { self.inner.as_mut() }
    }
}

unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB> for PageFrameAllocator{
    fn allocate_frame(&mut self) -> Option<x86_64::structures::paging::PhysFrame<Size4KiB>> {
        let page_base_addr = self.request_page().ok()?;

        PhysFrame::from_start_address(PhysAddr::new(page_base_addr as u64)).ok()
    }
}
