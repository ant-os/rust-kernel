#[macro_use]
use crate::{debug, endl};

use limine::{MemmapEntry, MemmapResponse, MemoryMapEntryType, NonNullPtr};

pub struct PageFrameAllocator {
    pub segment: Option<limine::NonNullPtr<limine::MemmapEntry>>,
}

impl PageFrameAllocator {
    pub fn from_response(resp: &MemmapResponse) -> PageFrameAllocator {
        if core::intrinsics::unlikely(resp.entry_count == 0) {
            unimplemented!()
        }

        let mut largest_free_segment = None;
        let mut largest_free_segment_size = 0;
        let mut total_memory = 0;

        for entry in resp.memmap().iter() {
            if entry.typ == MemoryMapEntryType::Usable && entry.len > largest_free_segment_size {
                largest_free_segment = Some(entry);
                largest_free_segment_size = entry.len;
            }

            total_memory += entry.len;
        }

        debug!(
            crate::integer_to_string(largest_free_segment_size / 4095 / 8),
            endl!()
        );
        debug!(crate::integer_to_string(total_memory / 4095), endl!());

        let bitmap = Self::alloc_storage(largest_free_segment, (total_memory / 4095) as usize);

        debug!(crate::integer_to_string(bitmap.len()));

        PageFrameAllocator { segment: None }
    }

    pub fn alloc_storage(
        segment: Option<&NonNullPtr<MemmapEntry>>,
        pages: usize,
    ) -> bitmap::Bitmap<PtrWrapper<[usize]>, bitmap::OneBit> {
        if core::intrinsics::unlikely(segment.is_none()) {
            unimplemented!()
        }

        if let Some(_seg) = segment {
            // This actually works...
            return unsafe {
                bitmap::Bitmap::<PtrWrapper<[usize]>, bitmap::OneBit>::from_storage(
                    pages + 1,
                    (),
                    PtrWrapper::<[usize]>::from_raw(core::slice::from_raw_parts_mut(
                        _seg.base as *mut usize,
                        pages + 1,
                    )),
                )
                .unwrap()
            };
        }

        unreachable!();
    }
}

struct PtrWrapper<T: ?Sized>(*mut T);

impl<T: ?Sized> PtrWrapper<T> {
    pub unsafe fn from_raw(_val: &mut T) -> PtrWrapper<T> {
        Self(&mut *_val as *mut T)
    }
}

impl bitmap::Storage for PtrWrapper<[usize]> {
    fn as_ref(&self) -> &[usize] {
        unsafe { &*self.0 }
    }

    fn as_mut(&mut self) -> &mut [usize] {
        unsafe { &mut *self.0 }
    }
}
