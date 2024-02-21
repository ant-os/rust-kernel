use core::alloc::AllocError;
use core::any::type_name_of_val;
use core::cell::{self, Cell, RefCell, UnsafeCell};
use core::hint::unreachable_unchecked;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::Ordering::SeqCst;
use core::sync::atomic::{AtomicBool, AtomicPtr};
use core::{alloc::Layout, cell::SyncUnsafeCell, ptr::null_mut, sync::atomic::AtomicUsize};
use crate::kprint;
use crate::paging::{frame_allocator, pf_allocator};
pub use alloc::*;
use limine::NonNullPtr;

pub extern crate alloc;

const ARENA_SIZE: usize = 558 * 1024;
const MAX_SUPPORTED_ALIGN: usize = 4096;
const FREE_BLOCKS_SIZE: usize = 8;

#[repr(C)]
pub struct UnsafePtrArray<T: ?Sized>(*mut T, usize);

impl<T: ?Sized> UnsafePtrArray<T> {
    pub fn new(value: *mut T, size: usize) -> Self {
        Self(unsafe { core::mem::transmute(value) }, size)
    }

    pub unsafe fn get(&self) -> *mut T {
        self.0
    }
}

#[repr(C, align(4096))]
pub struct KernelAllocator {
    arena: SyncUnsafeCell<[u8; ARENA_SIZE]>,
    is_initialized: AtomicBool,
    size: AtomicUsize,
    remaining: AtomicUsize,
}

static mut FREE_BLOCK_STORAGE: heapless::Vec<(usize, usize), FREE_BLOCKS_SIZE> =
    heapless::Vec::<(usize, usize), FREE_BLOCKS_SIZE>::new();

impl KernelAllocator {
    const unsafe fn new() -> Self {
        KernelAllocator {
            arena: SyncUnsafeCell::<[u8; ARENA_SIZE]>::new([0x0; ARENA_SIZE]),
            is_initialized: AtomicBool::new(false),
            size: AtomicUsize::new(ARENA_SIZE),
            remaining: AtomicUsize::new(ARENA_SIZE),
        }
    }

    pub const unsafe fn assume_init(&'_ self) { /* empty */ }

    pub fn get_arena_size(&self) -> usize {
        self.size.load(core::sync::atomic::Ordering::Relaxed)
    }

    pub fn initialize(&mut self) -> Result<(), frame_allocator::Error> {
        let size = self.size.load(SeqCst);
        let arena = pf_allocator().request_memory_area(size)?;

        self.is_initialized.store(true, SeqCst);

        // log::info!("Kernel Allocator Initialized! Initial allocator arena size: {}\n(\n{:#?}\n)\n", type_name_of_val(&self.size), self.size.load(SeqCst));

        Ok(())
    }

    pub unsafe fn get_arena(&mut self) -> *mut u8 {
        self.arena.get_mut().as_mut_ptr()
    }
}

#[doc(hidden)]
unsafe fn __rustc_dealloc(ptr: *mut u8, layout: core::alloc::Layout) { /* dummy */
}

// Mark the Allocator as being thread-safe.
unsafe impl Sync for KernelAllocator {
    /* empty */
}

/// The Kernel's Global Allocator.
#[doc(hidden)]
#[repr(C, packed)]
struct GlobalAllocImpl;
unsafe impl Sync for GlobalAllocImpl {}

impl KernelAllocator {
    unsafe fn allocate(&mut self, layout: Layout) -> Result<*mut u8, core::alloc::AllocError> {
        if !self.is_initialized.load(SeqCst) {
            return Err(AllocError);
        }

        let size = layout.size();
        let align = layout.align();

        // `Layout` contract forbids making a `Layout` with align=0, or align not power of 2.
        // So we can safely use a mask to ensure alignment without worrying about UB.
        let align_mask_to_round_down = !(align - 1);

        if align > MAX_SUPPORTED_ALIGN {
            return Err(AllocError);
        }

        let mut allocated = 0;
        if self
            .remaining
            .fetch_update(SeqCst, SeqCst, |mut remaining| {
                if size > remaining {
                    return None;
                }
                remaining -= size;
                remaining &= align_mask_to_round_down;
                allocated = remaining;
                Some(remaining)
            })
            .is_err()
        {
            return Err(AllocError);
        };
        Ok(self.get_arena().cast::<u8>().add(allocated))
    }

    unsafe fn deallocate(&self, ptr: *mut u8, layout: Layout) {
        if !self.is_initialized.load(SeqCst) {
            return;
        }

        self.remaining.fetch_add(layout.size(), SeqCst);

        __rustc_dealloc(ptr, layout)
    }
}

unsafe impl alloc::alloc::GlobalAlloc for GlobalAllocImpl {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match KERNEL_ALLOCATOR.allocate(layout) {
            Ok(ptr) => ptr,
            Err(_) => null_mut(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        KERNEL_ALLOCATOR.deallocate(ptr, layout)
    }
}

#[doc(hidden)]
#[global_allocator]
pub static mut GLOBAL_ALLOC: GlobalAllocImpl = GlobalAllocImpl;

#[doc(hidden)]
pub(crate) static mut KERNEL_ALLOCATOR: KernelAllocator = unsafe { KernelAllocator::new() };
