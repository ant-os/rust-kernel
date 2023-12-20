#![feature(prelude_import)]
#![recursion_limit = "225"]
#![no_std]
#![no_main]
#![allow(deprecated, incomplete_features, internal_features)]
#![feature(
    panic_info_message,
    unboxed_closures,
    core_intrinsics,
    decl_macro,
    ptr_from_ref,
    inherent_associated_types,
    adt_const_params,
    abi_x86_interrupt,
    allocator_api,
    const_mut_refs,
    portable_simd,
    strict_provenance,
    sync_unsafe_cell,
    debug_closure_helpers,
    if_let_guard,
    let_chains,
    panic_internals,
    marker_trait_attr,
    asm_const,
    type_name_of_val,
    alloc_internals,
    lazy_cell,
)]
#[prelude_import]
use core::prelude::rust_2021::*;
#[macro_use]
extern crate core;
extern crate compiler_builtins as _;
extern crate alloc;
pub mod alloc_impl {
    use core::alloc::AllocError;
    use core::any::type_name_of_val;
    use core::cell::{self, Cell, RefCell, UnsafeCell};
    use core::mem::MaybeUninit;
    use core::ops::Deref;
    use core::ptr::NonNull;
    use core::sync::atomic::{AtomicPtr, AtomicBool};
    use core::{
        sync::atomic::AtomicUsize, cell::SyncUnsafeCell, alloc::Layout, ptr::null_mut,
    };
    use core::sync::atomic::Ordering::SeqCst;
    pub use alloc::*;
    use limine::NonNullPtr;
    use crate::kprint;
    use crate::paging::{pf_allocator, frame_allocator};
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
    static mut FREE_BLOCK_STORAGE: heapless::Vec<(usize, usize), FREE_BLOCKS_SIZE> = heapless::Vec::<
        (usize, usize),
        FREE_BLOCKS_SIZE,
    >::new();
    impl KernelAllocator {
        const unsafe fn new() -> Self {
            KernelAllocator {
                arena: SyncUnsafeCell::<[u8; ARENA_SIZE]>::new([0x0; ARENA_SIZE]),
                is_initialized: AtomicBool::new(false),
                size: AtomicUsize::new(ARENA_SIZE),
                remaining: AtomicUsize::new(ARENA_SIZE),
            }
        }
        pub const unsafe fn assume_init(&'_ self) {}
        pub fn get_arena_size(&self) -> usize {
            self.size.load(core::sync::atomic::Ordering::Relaxed)
        }
        pub fn initialize(&mut self) -> Result<(), frame_allocator::Error> {
            let size = self.size.load(SeqCst);
            let arena = pf_allocator().request_memory_area(size)?;
            self.is_initialized.store(true, SeqCst);
            Ok(())
        }
        pub unsafe fn get_arena(&mut self) -> *mut u8 {
            self.arena.get_mut().as_mut_ptr()
        }
    }
    #[doc(hidden)]
    unsafe fn __rustc_dealloc(ptr: *mut u8, layout: core::alloc::Layout) {}
    unsafe impl Sync for KernelAllocator {}
    /// The Kernel's Global Allocator.
    #[doc(hidden)]
    #[repr(C, packed)]
    struct GlobalAllocImpl;
    unsafe impl Sync for GlobalAllocImpl {}
    impl KernelAllocator {
        unsafe fn allocate(
            &mut self,
            layout: Layout,
        ) -> Result<*mut u8, core::alloc::AllocError> {
            if !self.is_initialized.load(SeqCst) {
                return Err(AllocError);
            }
            let size = layout.size();
            let align = layout.align();
            let align_mask_to_round_down = !(align - 1);
            if align > MAX_SUPPORTED_ALIGN {
                return Err(AllocError);
            }
            let mut allocated = 0;
            if self
                .remaining
                .fetch_update(
                    SeqCst,
                    SeqCst,
                    |mut remaining| {
                        if size > remaining {
                            return None;
                        }
                        remaining -= size;
                        remaining &= align_mask_to_round_down;
                        allocated = remaining;
                        Some(remaining)
                    },
                )
                .is_err()
            {
                return Err(AllocError);
            }
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
    pub static mut GLOBAL_ALLOC: GlobalAllocImpl = GlobalAllocImpl;
    const _: () = {
        #[rustc_std_internal_symbol]
        unsafe fn __rust_alloc(size: usize, align: usize) -> *mut u8 {
            ::core::alloc::GlobalAlloc::alloc(
                &GLOBAL_ALLOC,
                ::core::alloc::Layout::from_size_align_unchecked(size, align),
            )
        }
        #[rustc_std_internal_symbol]
        unsafe fn __rust_dealloc(ptr: *mut u8, size: usize, align: usize) -> () {
            ::core::alloc::GlobalAlloc::dealloc(
                &GLOBAL_ALLOC,
                ptr,
                ::core::alloc::Layout::from_size_align_unchecked(size, align),
            )
        }
        #[rustc_std_internal_symbol]
        unsafe fn __rust_realloc(
            ptr: *mut u8,
            size: usize,
            align: usize,
            new_size: usize,
        ) -> *mut u8 {
            ::core::alloc::GlobalAlloc::realloc(
                &GLOBAL_ALLOC,
                ptr,
                ::core::alloc::Layout::from_size_align_unchecked(size, align),
                new_size,
            )
        }
        #[rustc_std_internal_symbol]
        unsafe fn __rust_alloc_zeroed(size: usize, align: usize) -> *mut u8 {
            ::core::alloc::GlobalAlloc::alloc_zeroed(
                &GLOBAL_ALLOC,
                ::core::alloc::Layout::from_size_align_unchecked(size, align),
            )
        }
    };
    #[doc(hidden)]
    pub(crate) static mut KERNEL_ALLOCATOR: KernelAllocator = unsafe {
        KernelAllocator::new()
    };
}
pub mod bitmap_font {
    pub type BitmapChar = [u16; 8];
    pub type BitmapFont = [BitmapChar; 128];
    pub trait DisplayChar {
        fn is_set(&self, x: usize, y: usize) -> bool;
    }
    impl DisplayChar for BitmapChar {
        fn is_set(&self, x: usize, y: usize) -> bool {
            (self[y] & 1 << (x as i8)) != 0
        }
    }
}
pub mod common {
    pub mod consts {
        pub const PAGE_SIZE: u64 = 4096;
        pub const INVALID_PAGE_STATE: u64 = 2;
    }
    pub mod io {
        use core::arch::asm;
        #[inline]
        pub unsafe fn outb(port: u16, value: u8) {
            asm!(
                "out dx, al", in ("dx") port, in ("al") value, options(nomem,
                preserves_flags, nostack)
            );
        }
        #[inline]
        pub unsafe fn inb(port: u16) -> u8 {
            let value: u8;
            asm!(
                "in al, dx", out("al") value, in ("dx") port, options(nomem,
                preserves_flags, nostack)
            );
            value
        }
        #[inline(always)]
        pub unsafe fn io_wait() {}
    }
    pub mod macros {}
    use core::{
        simd::ptr::SimdConstPtr, ptr::NonNull, mem::{size_of_val, size_of},
        sync::atomic::AtomicPtr, ops::Deref,
    };
    pub use limine::*;
    pub mod idt {
        //! Interrupt Descriptor Table
        use core::{arch::asm, alloc::Layout, sync::atomic::AtomicPtr};
        use bitflags::bitflags;
        use crate::{alloc_impl::KERNEL_ALLOCATOR, paging::pt_manager};
        const GDT_KERNEL_CODE: u16 = 0x8;
        pub type IdtEntries = [IdtEntry; 256];
        #[repr(C)]
        pub struct Idt {
            pub entries: IdtEntries,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Idt {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "Idt",
                    "entries",
                    &&self.entries,
                )
            }
        }
        impl Idt {
            pub const fn new() -> Self {
                Self {
                    entries: [IdtEntry::new(); 256],
                }
            }
            pub unsafe fn load_into(&self) {
                super::lidt(&self.pointer())
            }
            pub fn pointer(&self) -> super::DescriptorTablePointer {
                use core::mem::size_of;
                super::DescriptorTablePointer {
                    base: self as *const _ as u64,
                    limit: (size_of::<Self>() - 1) as u16,
                }
            }
        }
        pub struct IdtFlags(<IdtFlags as ::bitflags::__private::PublicFlags>::Internal);
        impl IdtFlags {
            #[allow(deprecated, non_upper_case_globals)]
            pub const PRESENT: Self = Self::from_bits_retain(1 << 7);
            #[allow(deprecated, non_upper_case_globals)]
            pub const RING_0: Self = Self::from_bits_retain(0 << 5);
            #[allow(deprecated, non_upper_case_globals)]
            pub const RING_1: Self = Self::from_bits_retain(1 << 5);
            #[allow(deprecated, non_upper_case_globals)]
            pub const RING_2: Self = Self::from_bits_retain(2 << 5);
            #[allow(deprecated, non_upper_case_globals)]
            pub const RING_3: Self = Self::from_bits_retain(3 << 5);
            #[allow(deprecated, non_upper_case_globals)]
            pub const SS: Self = Self::from_bits_retain(1 << 4);
            #[allow(deprecated, non_upper_case_globals)]
            pub const INTERRUPT: Self = Self::from_bits_retain(0xE);
            #[allow(deprecated, non_upper_case_globals)]
            pub const TRAP: Self = Self::from_bits_retain(0xF);
        }
        impl ::bitflags::Flags for IdtFlags {
            const FLAGS: &'static [::bitflags::Flag<IdtFlags>] = &[
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("PRESENT", IdtFlags::PRESENT)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("RING_0", IdtFlags::RING_0)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("RING_1", IdtFlags::RING_1)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("RING_2", IdtFlags::RING_2)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("RING_3", IdtFlags::RING_3)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("SS", IdtFlags::SS)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("INTERRUPT", IdtFlags::INTERRUPT)
                },
                {
                    #[allow(deprecated, non_upper_case_globals)]
                    ::bitflags::Flag::new("TRAP", IdtFlags::TRAP)
                },
            ];
            type Bits = u8;
            fn bits(&self) -> u8 {
                IdtFlags::bits(self)
            }
            fn from_bits_retain(bits: u8) -> IdtFlags {
                IdtFlags::from_bits_retain(bits)
            }
        }
        #[allow(
            dead_code,
            deprecated,
            unused_doc_comments,
            unused_attributes,
            unused_mut,
            unused_imports,
            non_upper_case_globals,
            clippy::assign_op_pattern,
            clippy::indexing_slicing,
            clippy::same_name_method,
            clippy::iter_without_into_iter,
        )]
        const _: () = {
            #[repr(transparent)]
            pub struct InternalBitFlags(u8);
            #[automatically_derived]
            impl ::core::clone::Clone for InternalBitFlags {
                #[inline]
                fn clone(&self) -> InternalBitFlags {
                    let _: ::core::clone::AssertParamIsClone<u8>;
                    *self
                }
            }
            #[automatically_derived]
            impl ::core::marker::Copy for InternalBitFlags {}
            #[automatically_derived]
            impl ::core::marker::StructuralPartialEq for InternalBitFlags {}
            #[automatically_derived]
            impl ::core::cmp::PartialEq for InternalBitFlags {
                #[inline]
                fn eq(&self, other: &InternalBitFlags) -> bool {
                    self.0 == other.0
                }
            }
            #[automatically_derived]
            impl ::core::marker::StructuralEq for InternalBitFlags {}
            #[automatically_derived]
            impl ::core::cmp::Eq for InternalBitFlags {
                #[inline]
                #[doc(hidden)]
                #[coverage(off)]
                fn assert_receiver_is_total_eq(&self) -> () {
                    let _: ::core::cmp::AssertParamIsEq<u8>;
                }
            }
            #[automatically_derived]
            impl ::core::cmp::PartialOrd for InternalBitFlags {
                #[inline]
                fn partial_cmp(
                    &self,
                    other: &InternalBitFlags,
                ) -> ::core::option::Option<::core::cmp::Ordering> {
                    ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Ord for InternalBitFlags {
                #[inline]
                fn cmp(&self, other: &InternalBitFlags) -> ::core::cmp::Ordering {
                    ::core::cmp::Ord::cmp(&self.0, &other.0)
                }
            }
            #[automatically_derived]
            impl ::core::hash::Hash for InternalBitFlags {
                #[inline]
                fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                    ::core::hash::Hash::hash(&self.0, state)
                }
            }
            impl ::bitflags::__private::PublicFlags for IdtFlags {
                type Primitive = u8;
                type Internal = InternalBitFlags;
            }
            impl ::bitflags::__private::core::default::Default for InternalBitFlags {
                #[inline]
                fn default() -> Self {
                    InternalBitFlags::empty()
                }
            }
            impl ::bitflags::__private::core::fmt::Debug for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter<'_>,
                ) -> ::bitflags::__private::core::fmt::Result {
                    if self.is_empty() {
                        f.write_fmt(
                            format_args!("{0:#x}", <u8 as ::bitflags::Bits>::EMPTY),
                        )
                    } else {
                        ::bitflags::__private::core::fmt::Display::fmt(self, f)
                    }
                }
            }
            impl ::bitflags::__private::core::fmt::Display for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter<'_>,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::parser::to_writer(&IdtFlags(*self), f)
                }
            }
            impl ::bitflags::__private::core::str::FromStr for InternalBitFlags {
                type Err = ::bitflags::parser::ParseError;
                fn from_str(
                    s: &str,
                ) -> ::bitflags::__private::core::result::Result<Self, Self::Err> {
                    ::bitflags::parser::from_str::<IdtFlags>(s).map(|flags| flags.0)
                }
            }
            impl ::bitflags::__private::core::convert::AsRef<u8> for InternalBitFlags {
                fn as_ref(&self) -> &u8 {
                    &self.0
                }
            }
            impl ::bitflags::__private::core::convert::From<u8> for InternalBitFlags {
                fn from(bits: u8) -> Self {
                    Self::from_bits_retain(bits)
                }
            }
            #[allow(dead_code, deprecated, unused_attributes)]
            impl InternalBitFlags {
                /// Get a flags value with all bits unset.
                #[inline]
                pub const fn empty() -> Self {
                    { Self(<u8 as ::bitflags::Bits>::EMPTY) }
                }
                /// Get a flags value with all known bits set.
                #[inline]
                pub const fn all() -> Self {
                    {
                        let mut truncated = <u8 as ::bitflags::Bits>::EMPTY;
                        let mut i = 0;
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        {
                            {
                                let flag = <IdtFlags as ::bitflags::Flags>::FLAGS[i]
                                    .value()
                                    .bits();
                                truncated = truncated | flag;
                                i += 1;
                            }
                        };
                        let _ = i;
                        Self::from_bits_retain(truncated)
                    }
                }
                /// Get the underlying bits value.
                ///
                /// The returned value is exactly the bits set in this flags value.
                #[inline]
                pub const fn bits(&self) -> u8 {
                    let f = self;
                    { f.0 }
                }
                /// Convert from a bits value.
                ///
                /// This method will return `None` if any unknown bits are set.
                #[inline]
                pub const fn from_bits(
                    bits: u8,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let bits = bits;
                    {
                        let truncated = Self::from_bits_truncate(bits).0;
                        if truncated == bits {
                            ::bitflags::__private::core::option::Option::Some(Self(bits))
                        } else {
                            ::bitflags::__private::core::option::Option::None
                        }
                    }
                }
                /// Convert from a bits value, unsetting any unknown bits.
                #[inline]
                pub const fn from_bits_truncate(bits: u8) -> Self {
                    let bits = bits;
                    { Self(bits & Self::all().bits()) }
                }
                /// Convert from a bits value exactly.
                #[inline]
                pub const fn from_bits_retain(bits: u8) -> Self {
                    let bits = bits;
                    { Self(bits) }
                }
                /// Get a flags value with the bits of a flag with the given name set.
                ///
                /// This method will return `None` if `name` is empty or doesn't
                /// correspond to any named flag.
                #[inline]
                pub fn from_name(
                    name: &str,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let name = name;
                    {
                        {
                            if name == "PRESENT" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::PRESENT.bits()),
                                );
                            }
                        };
                        {
                            if name == "RING_0" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::RING_0.bits()),
                                );
                            }
                        };
                        {
                            if name == "RING_1" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::RING_1.bits()),
                                );
                            }
                        };
                        {
                            if name == "RING_2" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::RING_2.bits()),
                                );
                            }
                        };
                        {
                            if name == "RING_3" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::RING_3.bits()),
                                );
                            }
                        };
                        {
                            if name == "SS" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::SS.bits()),
                                );
                            }
                        };
                        {
                            if name == "INTERRUPT" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::INTERRUPT.bits()),
                                );
                            }
                        };
                        {
                            if name == "TRAP" {
                                return ::bitflags::__private::core::option::Option::Some(
                                    Self(IdtFlags::TRAP.bits()),
                                );
                            }
                        };
                        let _ = name;
                        ::bitflags::__private::core::option::Option::None
                    }
                }
                /// Whether all bits in this flags value are unset.
                #[inline]
                pub const fn is_empty(&self) -> bool {
                    let f = self;
                    { f.bits() == <u8 as ::bitflags::Bits>::EMPTY }
                }
                /// Whether all known bits in this flags value are set.
                #[inline]
                pub const fn is_all(&self) -> bool {
                    let f = self;
                    { Self::all().bits() | f.bits() == f.bits() }
                }
                /// Whether any set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn intersects(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.bits() & other.bits() != <u8 as ::bitflags::Bits>::EMPTY }
                }
                /// Whether all set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn contains(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.bits() & other.bits() == other.bits() }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                pub fn insert(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits()).union(other);
                    }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `remove` won't truncate `other`, but the `!` operator will.
                #[inline]
                pub fn remove(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits()).difference(other);
                    }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                pub fn toggle(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    {
                        *f = Self::from_bits_retain(f.bits())
                            .symmetric_difference(other);
                    }
                }
                /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
                #[inline]
                pub fn set(&mut self, other: Self, value: bool) {
                    let f = self;
                    let other = other;
                    let value = value;
                    {
                        if value {
                            f.insert(other);
                        } else {
                            f.remove(other);
                        }
                    }
                }
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn intersection(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() & other.bits()) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn union(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() | other.bits()) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                #[must_use]
                pub const fn difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() & !other.bits()) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn symmetric_difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self::from_bits_retain(f.bits() ^ other.bits()) }
                }
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                #[must_use]
                pub const fn complement(self) -> Self {
                    let f = self;
                    { Self::from_bits_truncate(!f.bits()) }
                }
            }
            impl ::bitflags::__private::core::fmt::Binary for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::Binary::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::Octal for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::Octal::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::LowerHex for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::LowerHex::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::UpperHex for InternalBitFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::UpperHex::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::ops::BitOr for InternalBitFlags {
                type Output = Self;
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor(self, other: InternalBitFlags) -> Self {
                    self.union(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitOrAssign for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor_assign(&mut self, other: Self) {
                    self.insert(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitXor for InternalBitFlags {
                type Output = Self;
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor(self, other: Self) -> Self {
                    self.symmetric_difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitXorAssign for InternalBitFlags {
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor_assign(&mut self, other: Self) {
                    self.toggle(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitAnd for InternalBitFlags {
                type Output = Self;
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand(self, other: Self) -> Self {
                    self.intersection(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitAndAssign for InternalBitFlags {
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand_assign(&mut self, other: Self) {
                    *self = Self::from_bits_retain(self.bits()).intersection(other);
                }
            }
            impl ::bitflags::__private::core::ops::Sub for InternalBitFlags {
                type Output = Self;
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub(self, other: Self) -> Self {
                    self.difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::SubAssign for InternalBitFlags {
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub_assign(&mut self, other: Self) {
                    self.remove(other);
                }
            }
            impl ::bitflags::__private::core::ops::Not for InternalBitFlags {
                type Output = Self;
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                fn not(self) -> Self {
                    self.complement()
                }
            }
            impl ::bitflags::__private::core::iter::Extend<InternalBitFlags>
            for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn extend<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(&mut self, iterator: T) {
                    for item in iterator {
                        self.insert(item)
                    }
                }
            }
            impl ::bitflags::__private::core::iter::FromIterator<InternalBitFlags>
            for InternalBitFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn from_iter<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(iterator: T) -> Self {
                    use ::bitflags::__private::core::iter::Extend;
                    let mut result = Self::empty();
                    result.extend(iterator);
                    result
                }
            }
            impl InternalBitFlags {
                /// Yield a set of contained flags values.
                ///
                /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
                /// will be yielded together as a final flags value.
                #[inline]
                pub const fn iter(&self) -> ::bitflags::iter::Iter<IdtFlags> {
                    ::bitflags::iter::Iter::__private_const_new(
                        <IdtFlags as ::bitflags::Flags>::FLAGS,
                        IdtFlags::from_bits_retain(self.bits()),
                        IdtFlags::from_bits_retain(self.bits()),
                    )
                }
                /// Yield a set of contained named flags values.
                ///
                /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
                /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
                #[inline]
                pub const fn iter_names(&self) -> ::bitflags::iter::IterNames<IdtFlags> {
                    ::bitflags::iter::IterNames::__private_const_new(
                        <IdtFlags as ::bitflags::Flags>::FLAGS,
                        IdtFlags::from_bits_retain(self.bits()),
                        IdtFlags::from_bits_retain(self.bits()),
                    )
                }
            }
            impl ::bitflags::__private::core::iter::IntoIterator for InternalBitFlags {
                type Item = IdtFlags;
                type IntoIter = ::bitflags::iter::Iter<IdtFlags>;
                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
            impl InternalBitFlags {
                /// Returns a mutable reference to the raw value of the flags currently stored.
                #[inline]
                pub fn bits_mut(&mut self) -> &mut u8 {
                    &mut self.0
                }
            }
            #[allow(dead_code, deprecated, unused_attributes)]
            impl IdtFlags {
                /// Get a flags value with all bits unset.
                #[inline]
                pub const fn empty() -> Self {
                    { Self(InternalBitFlags::empty()) }
                }
                /// Get a flags value with all known bits set.
                #[inline]
                pub const fn all() -> Self {
                    { Self(InternalBitFlags::all()) }
                }
                /// Get the underlying bits value.
                ///
                /// The returned value is exactly the bits set in this flags value.
                #[inline]
                pub const fn bits(&self) -> u8 {
                    let f = self;
                    { f.0.bits() }
                }
                /// Convert from a bits value.
                ///
                /// This method will return `None` if any unknown bits are set.
                #[inline]
                pub const fn from_bits(
                    bits: u8,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let bits = bits;
                    {
                        match InternalBitFlags::from_bits(bits) {
                            ::bitflags::__private::core::option::Option::Some(bits) => {
                                ::bitflags::__private::core::option::Option::Some(
                                    Self(bits),
                                )
                            }
                            ::bitflags::__private::core::option::Option::None => {
                                ::bitflags::__private::core::option::Option::None
                            }
                        }
                    }
                }
                /// Convert from a bits value, unsetting any unknown bits.
                #[inline]
                pub const fn from_bits_truncate(bits: u8) -> Self {
                    let bits = bits;
                    { Self(InternalBitFlags::from_bits_truncate(bits)) }
                }
                /// Convert from a bits value exactly.
                #[inline]
                pub const fn from_bits_retain(bits: u8) -> Self {
                    let bits = bits;
                    { Self(InternalBitFlags::from_bits_retain(bits)) }
                }
                /// Get a flags value with the bits of a flag with the given name set.
                ///
                /// This method will return `None` if `name` is empty or doesn't
                /// correspond to any named flag.
                #[inline]
                pub fn from_name(
                    name: &str,
                ) -> ::bitflags::__private::core::option::Option<Self> {
                    let name = name;
                    {
                        match InternalBitFlags::from_name(name) {
                            ::bitflags::__private::core::option::Option::Some(bits) => {
                                ::bitflags::__private::core::option::Option::Some(
                                    Self(bits),
                                )
                            }
                            ::bitflags::__private::core::option::Option::None => {
                                ::bitflags::__private::core::option::Option::None
                            }
                        }
                    }
                }
                /// Whether all bits in this flags value are unset.
                #[inline]
                pub const fn is_empty(&self) -> bool {
                    let f = self;
                    { f.0.is_empty() }
                }
                /// Whether all known bits in this flags value are set.
                #[inline]
                pub const fn is_all(&self) -> bool {
                    let f = self;
                    { f.0.is_all() }
                }
                /// Whether any set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn intersects(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.0.intersects(other.0) }
                }
                /// Whether all set bits in a source flags value are also set in a target flags value.
                #[inline]
                pub const fn contains(&self, other: Self) -> bool {
                    let f = self;
                    let other = other;
                    { f.0.contains(other.0) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                pub fn insert(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.insert(other.0) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `remove` won't truncate `other`, but the `!` operator will.
                #[inline]
                pub fn remove(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.remove(other.0) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                pub fn toggle(&mut self, other: Self) {
                    let f = self;
                    let other = other;
                    { f.0.toggle(other.0) }
                }
                /// Call `insert` when `value` is `true` or `remove` when `value` is `false`.
                #[inline]
                pub fn set(&mut self, other: Self, value: bool) {
                    let f = self;
                    let other = other;
                    let value = value;
                    { f.0.set(other.0, value) }
                }
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn intersection(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.intersection(other.0)) }
                }
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn union(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.union(other.0)) }
                }
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                #[must_use]
                pub const fn difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.difference(other.0)) }
                }
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                #[must_use]
                pub const fn symmetric_difference(self, other: Self) -> Self {
                    let f = self;
                    let other = other;
                    { Self(f.0.symmetric_difference(other.0)) }
                }
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                #[must_use]
                pub const fn complement(self) -> Self {
                    let f = self;
                    { Self(f.0.complement()) }
                }
            }
            impl ::bitflags::__private::core::fmt::Binary for IdtFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::Binary::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::Octal for IdtFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::Octal::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::LowerHex for IdtFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::LowerHex::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::fmt::UpperHex for IdtFlags {
                fn fmt(
                    &self,
                    f: &mut ::bitflags::__private::core::fmt::Formatter,
                ) -> ::bitflags::__private::core::fmt::Result {
                    ::bitflags::__private::core::fmt::UpperHex::fmt(&self.0, f)
                }
            }
            impl ::bitflags::__private::core::ops::BitOr for IdtFlags {
                type Output = Self;
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor(self, other: IdtFlags) -> Self {
                    self.union(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitOrAssign for IdtFlags {
                /// The bitwise or (`|`) of the bits in two flags values.
                #[inline]
                fn bitor_assign(&mut self, other: Self) {
                    self.insert(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitXor for IdtFlags {
                type Output = Self;
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor(self, other: Self) -> Self {
                    self.symmetric_difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitXorAssign for IdtFlags {
                /// The bitwise exclusive-or (`^`) of the bits in two flags values.
                #[inline]
                fn bitxor_assign(&mut self, other: Self) {
                    self.toggle(other);
                }
            }
            impl ::bitflags::__private::core::ops::BitAnd for IdtFlags {
                type Output = Self;
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand(self, other: Self) -> Self {
                    self.intersection(other)
                }
            }
            impl ::bitflags::__private::core::ops::BitAndAssign for IdtFlags {
                /// The bitwise and (`&`) of the bits in two flags values.
                #[inline]
                fn bitand_assign(&mut self, other: Self) {
                    *self = Self::from_bits_retain(self.bits()).intersection(other);
                }
            }
            impl ::bitflags::__private::core::ops::Sub for IdtFlags {
                type Output = Self;
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub(self, other: Self) -> Self {
                    self.difference(other)
                }
            }
            impl ::bitflags::__private::core::ops::SubAssign for IdtFlags {
                /// The intersection of a source flags value with the complement of a target flags value (`&!`).
                ///
                /// This method is not equivalent to `self & !other` when `other` has unknown bits set.
                /// `difference` won't truncate `other`, but the `!` operator will.
                #[inline]
                fn sub_assign(&mut self, other: Self) {
                    self.remove(other);
                }
            }
            impl ::bitflags::__private::core::ops::Not for IdtFlags {
                type Output = Self;
                /// The bitwise negation (`!`) of the bits in a flags value, truncating the result.
                #[inline]
                fn not(self) -> Self {
                    self.complement()
                }
            }
            impl ::bitflags::__private::core::iter::Extend<IdtFlags> for IdtFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn extend<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(&mut self, iterator: T) {
                    for item in iterator {
                        self.insert(item)
                    }
                }
            }
            impl ::bitflags::__private::core::iter::FromIterator<IdtFlags> for IdtFlags {
                /// The bitwise or (`|`) of the bits in each flags value.
                fn from_iter<
                    T: ::bitflags::__private::core::iter::IntoIterator<Item = Self>,
                >(iterator: T) -> Self {
                    use ::bitflags::__private::core::iter::Extend;
                    let mut result = Self::empty();
                    result.extend(iterator);
                    result
                }
            }
            impl IdtFlags {
                /// Yield a set of contained flags values.
                ///
                /// Each yielded flags value will correspond to a defined named flag. Any unknown bits
                /// will be yielded together as a final flags value.
                #[inline]
                pub const fn iter(&self) -> ::bitflags::iter::Iter<IdtFlags> {
                    ::bitflags::iter::Iter::__private_const_new(
                        <IdtFlags as ::bitflags::Flags>::FLAGS,
                        IdtFlags::from_bits_retain(self.bits()),
                        IdtFlags::from_bits_retain(self.bits()),
                    )
                }
                /// Yield a set of contained named flags values.
                ///
                /// This method is like [`iter`](#method.iter), except only yields bits in contained named flags.
                /// Any unknown bits, or bits not corresponding to a contained flag will not be yielded.
                #[inline]
                pub const fn iter_names(&self) -> ::bitflags::iter::IterNames<IdtFlags> {
                    ::bitflags::iter::IterNames::__private_const_new(
                        <IdtFlags as ::bitflags::Flags>::FLAGS,
                        IdtFlags::from_bits_retain(self.bits()),
                        IdtFlags::from_bits_retain(self.bits()),
                    )
                }
            }
            impl ::bitflags::__private::core::iter::IntoIterator for IdtFlags {
                type Item = IdtFlags;
                type IntoIter = ::bitflags::iter::Iter<IdtFlags>;
                fn into_iter(self) -> Self::IntoIter {
                    self.iter()
                }
            }
        };
        #[repr(packed)]
        pub struct IdtEntry {
            offsetl: u16,
            selector: u16,
            zero: u8,
            attribute: u8,
            offsetm: u16,
            offseth: u32,
            _zero2: u32,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for IdtEntry {}
        #[automatically_derived]
        impl ::core::clone::Clone for IdtEntry {
            #[inline]
            fn clone(&self) -> IdtEntry {
                let _: ::core::clone::AssertParamIsClone<u16>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                let _: ::core::clone::AssertParamIsClone<u32>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for IdtEntry {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let names: &'static _ = &[
                    "offsetl",
                    "selector",
                    "zero",
                    "attribute",
                    "offsetm",
                    "offseth",
                    "_zero2",
                ];
                let values: &[&dyn ::core::fmt::Debug] = &[
                    &{ self.offsetl },
                    &{ self.selector },
                    &{ self.zero },
                    &{ self.attribute },
                    &{ self.offsetm },
                    &{ self.offseth },
                    &&{ self._zero2 },
                ];
                ::core::fmt::Formatter::debug_struct_fields_finish(
                    f,
                    "IdtEntry",
                    names,
                    values,
                )
            }
        }
        #[automatically_derived]
        impl ::core::default::Default for IdtEntry {
            #[inline]
            fn default() -> IdtEntry {
                IdtEntry {
                    offsetl: ::core::default::Default::default(),
                    selector: ::core::default::Default::default(),
                    zero: ::core::default::Default::default(),
                    attribute: ::core::default::Default::default(),
                    offsetm: ::core::default::Default::default(),
                    offseth: ::core::default::Default::default(),
                    _zero2: ::core::default::Default::default(),
                }
            }
        }
        impl IdtEntry {
            pub const fn new() -> IdtEntry {
                IdtEntry {
                    offsetl: 0,
                    selector: 0,
                    zero: 0,
                    attribute: 0,
                    offsetm: 0,
                    offseth: 0,
                    _zero2: 0,
                }
            }
            pub fn set_flags(&mut self, flags: IdtFlags) {
                self.attribute = flags.bits();
            }
            pub fn set_ist(&mut self, ist: u8) {
                match (&(ist & 0x07), &ist) {
                    (left_val, right_val) => {
                        if !(*left_val == *right_val) {
                            let kind = ::core::panicking::AssertKind::Eq;
                            ::core::panicking::assert_failed(
                                kind,
                                &*left_val,
                                &*right_val,
                                ::core::option::Option::Some(
                                    format_args!("interrupt stack table must be within 0..=7"),
                                ),
                            );
                        }
                    }
                };
                self.zero &= 0xF8;
                self.zero |= ist;
            }
            pub fn set_offset(&mut self, selector: u16, base: usize) {
                self.selector = selector;
                self.offsetl = base as u16;
                self.offsetm = (base >> 16) as u16;
                self.offseth = ((base as u64) >> 32) as u32;
            }
            pub fn set_func(&mut self, func: unsafe extern fn()) {
                self.set_flags(
                    IdtFlags::PRESENT | IdtFlags::RING_0 | IdtFlags::INTERRUPT,
                );
                self.set_offset(0x8, func as usize);
            }
        }
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        pub struct KERNEL_IDT {
            __private_field: (),
        }
        #[doc(hidden)]
        pub static KERNEL_IDT: KERNEL_IDT = KERNEL_IDT { __private_field: () };
        impl ::lazy_static::__Deref for KERNEL_IDT {
            type Target = AtomicPtr<Idt>;
            fn deref(&self) -> &AtomicPtr<Idt> {
                #[inline(always)]
                fn __static_ref_initialize() -> AtomicPtr<Idt> {
                    unsafe {
                        KERNEL_ALLOCATOR.assume_init();
                        let idt = alloc::boxed::Box::<
                            Idt,
                        >::into_raw(alloc::boxed::Box::<Idt>::new(Idt::new()));
                        AtomicPtr::new(idt)
                    }
                }
                #[inline(always)]
                fn __stability() -> &'static AtomicPtr<Idt> {
                    static LAZY: ::lazy_static::lazy::Lazy<AtomicPtr<Idt>> = ::lazy_static::lazy::Lazy::INIT;
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl ::lazy_static::LazyStatic for KERNEL_IDT {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
    }
    pub mod gdt {
        //! Global Descriptor Table
        use super::{AtomicRef, DescriptorTablePointer};
        pub const GDT_NULL: usize = 0;
        pub const GDT_KERNEL_CODE: usize = 1;
        pub const GDT_KERNEL_DATA: usize = 2;
        pub const GDT_USER_CODE32_UNUSED: usize = 3;
        pub const GDT_USER_DATA: usize = 4;
        pub const GDT_USER_CODE: usize = 5;
        pub const GDT_TSS: usize = 6;
        pub const GDT_TSS_HIGH: usize = 7;
        pub const GDT_A_PRESENT: u8 = 1 << 7;
        pub const GDT_A_RING_0: u8 = 0 << 5;
        pub const GDT_A_RING_1: u8 = 1 << 5;
        pub const GDT_A_RING_2: u8 = 2 << 5;
        pub const GDT_A_RING_3: u8 = 3 << 5;
        pub const GDT_A_SYSTEM: u8 = 1 << 4;
        pub const GDT_A_EXECUTABLE: u8 = 1 << 3;
        pub const GDT_A_CONFORMING: u8 = 1 << 2;
        pub const GDT_A_PRIVILEGE: u8 = 1 << 1;
        pub const GDT_A_DIRTY: u8 = 1;
        pub const GDT_A_TSS_AVAIL: u8 = 0x9;
        pub const GDT_A_TSS_BUSY: u8 = 0xB;
        pub const GDT_F_PAGE_SIZE: u8 = 1 << 7;
        pub const GDT_F_PROTECTED_MODE: u8 = 1 << 6;
        pub const GDT_F_LONG_MODE: u8 = 1 << 5;
        #[repr(packed)]
        pub struct GdtEntry {
            pub limitl: u16,
            pub offsetl: u16,
            pub offsetm: u8,
            pub access: u8,
            pub flags_limith: u8,
            pub offseth: u8,
        }
        #[automatically_derived]
        impl ::core::marker::Copy for GdtEntry {}
        #[automatically_derived]
        impl ::core::clone::Clone for GdtEntry {
            #[inline]
            fn clone(&self) -> GdtEntry {
                let _: ::core::clone::AssertParamIsClone<u16>;
                let _: ::core::clone::AssertParamIsClone<u8>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for GdtEntry {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                let names: &'static _ = &[
                    "limitl",
                    "offsetl",
                    "offsetm",
                    "access",
                    "flags_limith",
                    "offseth",
                ];
                let values: &[&dyn ::core::fmt::Debug] = &[
                    &{ self.limitl },
                    &{ self.offsetl },
                    &{ self.offsetm },
                    &{ self.access },
                    &{ self.flags_limith },
                    &&{ self.offseth },
                ];
                ::core::fmt::Formatter::debug_struct_fields_finish(
                    f,
                    "GdtEntry",
                    names,
                    values,
                )
            }
        }
        impl GdtEntry {
            pub const fn new(offset: u32, limit: u32, access: u8, flags: u8) -> Self {
                GdtEntry {
                    limitl: limit as u16,
                    offsetl: offset as u16,
                    offsetm: (offset >> 16) as u8,
                    access,
                    flags_limith: flags & 0xF0 | ((limit >> 16) as u8) & 0x0F,
                    offseth: (offset >> 24) as u8,
                }
            }
            pub fn set_offset(&mut self, offset: u32) {
                self.offsetl = offset as u16;
                self.offsetm = (offset >> 16) as u8;
                self.offseth = (offset >> 24) as u8;
            }
            pub fn set_limit(&mut self, limit: u32) {
                self.limitl = limit as u16;
                self
                    .flags_limith = self.flags_limith & 0xF0
                    | ((limit >> 16) as u8) & 0x0F;
            }
        }
    }
    pub mod handler {
        use core::{cell::UnsafeCell, default};
        use core::marker;
        use core::sync::atomic::AtomicPtr;
        use alloc::sync::Arc;
        use limine::NonNullPtr;
        use lock_api::{RwLock, GuardSend};
        #[marker]
        pub trait Handler {}
        pub fn get_data_for_handler(handle: Handle) -> Option<*mut dyn Handler> {
            None
        }
        #[repr(transparent)]
        pub struct Handle {
            pub(self) inner: u64,
        }
        impl Handle {
            pub fn new(offset: u64) -> Self {
                Self { inner: offset }
            }
        }
    }
    pub type Unit = ();
    pub use x86_64::structures::idt::ExceptionVector;
    pub use x86_64::structures::idt::InterruptStackFrame;
    ///A [AtomicPtr] wrapper that implements [Deref].
    #[repr(transparent)]
    struct AtomicRef<T: Sized> {
        inner: AtomicPtr<T>,
    }
    unsafe impl<T: Sized> Sync for AtomicRef<T> {}
    /// It's just an [AtomicPtr] internally so it's "thread-safe".
    impl<T: Sized> AtomicRef<T> {
        ///Create a new [AtomicRef].
        fn new(inner: *mut T) -> Self {
            Self {
                inner: AtomicPtr::new(inner),
            }
        }
    }
    impl<T: Sized> Deref for AtomicRef<T> {
        type Target = T;
        ///Loads(Relaxed) and then Dereferences the pointer stored by the inner [AtomicRef]. **Panics** when the inner pointer is null.
        fn deref(&self) -> &Self::Target {
            unsafe {
                self.inner
                    .load(core::sync::atomic::Ordering::Relaxed)
                    .as_ref()
                    .expect("AtomicPtr was null.")
            }
        }
    }
    ///Utility Trait
    pub(crate) unsafe trait TransmuteIntoPointer {
        /// Calls [core::intrinsics::transmute_unchecked<Self, *mut T>].
        #[inline(always)]
        unsafe fn ptr<T: Sized>(self) -> *mut T
        where
            Self: Sized,
        {
            core::intrinsics::transmute_unchecked::<Self, *mut T>(self)
        }
    }
    ///Utility Trait
    pub(crate) unsafe trait TransmuteInto<T: Sized> {
        /// Calls [core::intrinsics::transmute_unchecked<Self, T>]
        #[inline(always)]
        unsafe fn transmute(self) -> T
        where
            Self: Sized,
        {
            core::intrinsics::transmute_unchecked::<Self, T>(self)
        }
    }
    unsafe impl TransmuteIntoPointer for usize {}
    unsafe impl<T: Sized> TransmuteInto<NonNullPtr<T>> for NonNull<T> {}
    unsafe impl<T: Sized> TransmuteInto<AtomicPtr<T>> for AtomicRef<T> {}
    unsafe impl TransmuteInto<ExceptionVector> for u8 {}
    #[cfg(target_pointer_width = "64")]
    unsafe impl TransmuteInto<u64> for usize {}
    unsafe impl<T: ?Sized> TransmuteInto<usize> for &'_ mut T {}
    unsafe impl<T: ?Sized> TransmuteInto<usize> for *mut T {}
    unsafe impl<T: ?Sized> TransmuteInto<usize> for *const T {}
    #[repr(C, packed(2))]
    pub struct DescriptorTablePointer {
        /// Size of the DT.
        pub limit: u16,
        /// Pointer to the memory region containing the DT.
        pub base: u64,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for DescriptorTablePointer {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "DescriptorTablePointer",
                "limit",
                &{ self.limit },
                "base",
                &&{ self.base },
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for DescriptorTablePointer {
        #[inline]
        fn clone(&self) -> DescriptorTablePointer {
            let _: ::core::clone::AssertParamIsClone<u16>;
            let _: ::core::clone::AssertParamIsClone<u64>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for DescriptorTablePointer {}
    #[inline]
    pub unsafe fn lidt(ptr: &DescriptorTablePointer) {
        unsafe {
            asm!("lidt [{0}]", in (reg) ptr, options(preserves_flags, nostack));
        }
    }
    #[inline]
    pub unsafe fn lgdt(ptr: &DescriptorTablePointer) {
        unsafe {
            asm!(
                "lgdt [{0}]", in (reg) ptr, options(readonly, preserves_flags, nostack)
            );
        }
    }
    pub unsafe fn _alloc_frame_as_mut_t<T: Sized>() -> Result<
        *mut T,
        super::paging::frame_allocator::Error,
    > {
        Ok(unsafe {
            core::intrinsics::transmute_unchecked::<
                *mut (),
                *mut T,
            >(crate::pf_allocator().request_page()? as *mut ())
        })
    }
    pub(crate) macro __kdebug_newline {
        () => { debug!("\n") }
    }
    pub macro endl {
        () => { "\n" }
    }
}
pub mod device {
    pub mod block {
        pub trait UnsafeBlockDevice {}
    }
    pub mod character {
        pub enum CharacterDeviceMode {
            Normal,
            Loopback,
        }
        pub trait UnsafeCharacterDevice
        where
            Self: super::GeneralDevice,
        {
            unsafe fn read_raw(&self) -> u8;
            unsafe fn write_raw(&self, data: u8);
            unsafe fn received(&self) -> bool;
            unsafe fn is_transmit_empty(&self) -> bool;
            unsafe fn test(&self) -> bool;
            unsafe fn init(&mut self) -> bool;
            fn set_mode(&mut self, mode: CharacterDeviceMode);
            fn get_mode(&self) -> CharacterDeviceMode;
        }
        pub trait TimedCharacterDevice
        where
            Self: UnsafeCharacterDevice,
        {
            unsafe fn read(&self) -> u8;
            unsafe fn write(&self, data: u8);
            unsafe fn wait(&self);
        }
    }
    pub mod input {
        pub trait UnsafeInputDevice {}
    }
    pub mod network {
        pub trait UnsafeNetworkDevice {}
    }
    pub enum Device<'r> {
        Character(&'r dyn character::UnsafeCharacterDevice),
        Block(&'r dyn block::UnsafeBlockDevice),
        Network(&'r dyn network::UnsafeNetworkDevice),
        Input(&'r dyn input::UnsafeInputDevice),
        General(&'r dyn GeneralDevice),
    }
    pub trait GeneralDevice {
        fn as_device(&self) -> crate::device::Device<'_>;
    }
}
pub mod framebuffer {
    pub use limine::Framebuffer;
}
pub mod graphics {
    use core::ops::Deref;
    use alloc::boxed::Box;
    #[repr(transparent)]
    pub struct Color(u32);
    #[automatically_derived]
    impl ::core::fmt::Debug for Color {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Color", &&self.0)
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Color {
        #[inline]
        fn clone(&self) -> Color {
            let _: ::core::clone::AssertParamIsClone<u32>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for Color {}
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Color {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Color {
        #[inline]
        fn eq(&self, other: &Color) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for Color {}
    #[automatically_derived]
    impl ::core::cmp::Eq for Color {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<u32>;
        }
    }
    const CHANNEL_MASK: u32 = 0b11000000;
    #[repr(packed)]
    pub struct RGBA {
        pub red: u8,
        pub green: u8,
        pub blue: u8,
        pub alpha: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for RGBA {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "RGBA",
                "red",
                &{ self.red },
                "green",
                &{ self.green },
                "blue",
                &{ self.blue },
                "alpha",
                &&{ self.alpha },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for RGBA {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for RGBA {
        #[inline]
        fn eq(&self, other: &RGBA) -> bool {
            ({ self.red }) == ({ other.red }) && ({ self.green }) == ({ other.green })
                && ({ self.blue }) == ({ other.blue })
                && ({ self.alpha }) == ({ other.alpha })
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for RGBA {}
    #[automatically_derived]
    impl ::core::cmp::Eq for RGBA {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<u8>;
        }
    }
    impl Color {
        #[inline(always)]
        pub const fn new(code: u32) -> Self {
            Self(code)
        }
        #[inline(always)]
        pub const fn rgb(&self) -> (u8, u8, u8) {
            (
                (self.0 & (CHANNEL_MASK >> 0)) as u8,
                (self.0 & (CHANNEL_MASK >> 2)) as u8,
                (self.0 & (CHANNEL_MASK >> 4)) as u8,
            )
        }
        #[inline(always)]
        pub const fn rgba(&self) -> (u8, u8, u8, u8) {
            (
                (self.0 & (CHANNEL_MASK >> 0)) as u8,
                (self.0 & (CHANNEL_MASK >> 2)) as u8,
                (self.0 & (CHANNEL_MASK >> 4)) as u8,
                (self.0 & (CHANNEL_MASK >> 6)) as u8,
            )
        }
        #[inline(always)]
        pub const fn from_rgb(r: u8, b: u8, g: u8) -> Self {
            Self::new(
                ((r as u32) << 6) | ((r as u32) << 4) | ((r as u32) << 2) | 0x000000FF,
            )
        }
        #[inline(always)]
        pub const fn inner(&self) -> u32 {
            self.0
        }
    }
    impl Deref for Color {
        type Target = RGBA;
        #[inline]
        fn deref(&'_ self) -> &'_ Self::Target {
            let rgba = self.rgba();
            unsafe { core::mem::transmute(self) }
        }
    }
}
pub mod kernel_logger {
    use core::sync::atomic::AtomicBool;
    use alloc::format;
    use log;
    use crate::{
        kprint, tty::KERNEL_CONSOLE, device::character::UnsafeCharacterDevice,
        renderer::Renderer,
    };
    pub struct KernelLogger {
        pub is_enabled: AtomicBool,
    }
    impl log::Log for KernelLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            self.is_enabled.load(core::sync::atomic::Ordering::Relaxed)
        }
        fn log(&self, record: &log::Record) {
            unsafe {
                crate::KERNEL_CONSOLE
                    .write_str(
                        {
                            let res = ::alloc::fmt::format(
                                format_args!(
                                    "[{0} {1}:{2}] {3}: {4}",
                                    record.level(),
                                    record.file().unwrap_or("<null>"),
                                    record.line().unwrap_or(0),
                                    record.target(),
                                    record.args(),
                                ),
                            );
                            res
                        }
                            .as_str(),
                    );
                crate::KERNEL_CONSOLE.newline();
            };
        }
        fn flush(&self) {}
    }
    pub(super) static mut KERNEL_LOGGER: KernelLogger = KernelLogger {
        is_enabled: AtomicBool::new(true),
    };
}
pub mod legacy_pic {
    //! Legacy PIC.
    const PIC1_COMMAND: u16 = 0x20;
    const PIC1_DATA: u16 = 0x21;
    const PIC2_COMMAND: u16 = 0xA0;
    const PIC2_DATA: u16 = 0xA1;
    const PIC_EOI: u8 = 0x20;
    const ICW1_INIT: u8 = 0x10;
    const ICW1_ICW4: u8 = 0x01;
    const ICW4_8086: u8 = 0x01;
    use bit::BitIndex;
    use super::io::{outb, inb, io_wait};
    pub struct PIC {
        pub(self) master_mask: u8,
        pub(self) slave_mask: u8,
    }
    pub enum Interrupt {
        PIT,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Interrupt {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(f, "PIT")
        }
    }
    impl Interrupt {
        pub fn enable_in(self, pic: &'_ mut PIC, value: bool) {
            match self {
                Interrupt::PIT => pic.master_mask.set_bit(0, value),
                _ => ::core::panicking::panic("not yet implemented"),
            };
        }
    }
    impl PIC {
        #[inline]
        pub const fn new() -> Self {
            Self {
                master_mask: 0x00,
                slave_mask: 0x00,
            }
        }
        pub unsafe fn remap(&self) {
            let a1 = inb(PIC1_DATA);
            io_wait();
            let a2 = inb(PIC2_DATA);
            io_wait();
            outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
            io_wait();
            outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);
            io_wait();
            outb(PIC1_DATA, 0x20);
            io_wait();
            outb(PIC2_DATA, 0x28);
            io_wait();
            outb(PIC1_DATA, 4);
            io_wait();
            outb(PIC2_DATA, 2);
            io_wait();
            outb(PIC1_DATA, ICW4_8086);
            io_wait();
            outb(PIC2_DATA, ICW4_8086);
            io_wait();
            outb(PIC1_DATA, a1);
            io_wait();
            outb(PIC2_DATA, a2);
        }
        pub unsafe fn needs_sync(&self) -> bool {
            (self.master_mask != inb(PIC1_DATA)) || (self.slave_mask != inb(PIC2_DATA))
        }
        pub unsafe fn sync(&self) {
            self.remap();
            outb(PIC1_DATA, self.master_mask);
            outb(PIC2_DATA, self.slave_mask);
        }
        pub fn enable(&mut self, int: Interrupt) {
            int.enable_in(self, false)
        }
        pub fn disable(&mut self, int: Interrupt) {
            int.enable_in(self, true)
        }
    }
    pub(crate) static mut PRIMARY_PIC: PIC = PIC::new();
}
pub mod memory {
    //! Common Memory Structures, e.g [VirtualAddress].
    use numtoa::NumToA as _;
    use x86_64::{VirtAddr, structures::paging::PageTable};
    /// Specific table to be used, needed on some architectures
    pub enum TableKind {
        /// Userspace page table
        User,
        /// Kernel page table
        Kernel,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for TableKind {
        #[inline]
        fn clone(&self) -> TableKind {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for TableKind {}
    #[automatically_derived]
    impl ::core::fmt::Debug for TableKind {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::write_str(
                f,
                match self {
                    TableKind::User => "User",
                    TableKind::Kernel => "Kernel",
                },
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for TableKind {}
    #[automatically_derived]
    impl ::core::cmp::Eq for TableKind {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for TableKind {
        #[inline]
        fn cmp(&self, other: &TableKind) -> ::core::cmp::Ordering {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for TableKind {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TableKind {
        #[inline]
        fn eq(&self, other: &TableKind) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for TableKind {
        #[inline]
        fn partial_cmp(
            &self,
            other: &TableKind,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
        }
    }
    pub(crate) const PHYSICAL_MEMORY_OFFSET: usize = 0xffffffff80000000;
    pub(crate) const PHYSICAL_BOOTLOADER_MEMORY_OFFSET: u64 = 0x00;
    pub(crate) unsafe fn active_level_4_table(
        physical_memory_offset: VirtAddr,
    ) -> &'static mut PageTable {
        use x86_64::registers::control::Cr3;
        let (level_4_table_frame, _) = Cr3::read();
        let phys = level_4_table_frame.start_address();
        let virt = VirtAddr::new_unsafe(phys.as_u64());
        let page_table_ptr: *mut PageTable = virt.as_mut_ptr();
        &mut *page_table_ptr
    }
    /// Physical memory address
    #[repr(transparent)]
    pub struct PhysicalAddress(usize);
    #[automatically_derived]
    impl ::core::clone::Clone for PhysicalAddress {
        #[inline]
        fn clone(&self) -> PhysicalAddress {
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for PhysicalAddress {}
    #[automatically_derived]
    impl ::core::fmt::Debug for PhysicalAddress {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_tuple_field1_finish(
                f,
                "PhysicalAddress",
                &&self.0,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for PhysicalAddress {}
    #[automatically_derived]
    impl ::core::cmp::Eq for PhysicalAddress {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<usize>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for PhysicalAddress {
        #[inline]
        fn cmp(&self, other: &PhysicalAddress) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for PhysicalAddress {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for PhysicalAddress {
        #[inline]
        fn eq(&self, other: &PhysicalAddress) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for PhysicalAddress {
        #[inline]
        fn partial_cmp(
            &self,
            other: &PhysicalAddress,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
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
        pub fn as_str<'a>(&self) -> &'a str {
            self.0.numtoa_str(16, unsafe { &mut crate::GENERIC_STATIC_BUFFER })
        }
        pub fn to_virtual(&self) -> VirtualAddress {
            VirtualAddress::new(PHYSICAL_MEMORY_OFFSET + (self.data() >> 12))
        }
    }
    /// Virtual memory address
    #[repr(transparent)]
    pub struct VirtualAddress(usize);
    #[automatically_derived]
    impl ::core::clone::Clone for VirtualAddress {
        #[inline]
        fn clone(&self) -> VirtualAddress {
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for VirtualAddress {}
    #[automatically_derived]
    impl ::core::marker::StructuralEq for VirtualAddress {}
    #[automatically_derived]
    impl ::core::cmp::Eq for VirtualAddress {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<usize>;
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for VirtualAddress {
        #[inline]
        fn cmp(&self, other: &VirtualAddress) -> ::core::cmp::Ordering {
            ::core::cmp::Ord::cmp(&self.0, &other.0)
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for VirtualAddress {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for VirtualAddress {
        #[inline]
        fn eq(&self, other: &VirtualAddress) -> bool {
            self.0 == other.0
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for VirtualAddress {
        #[inline]
        fn partial_cmp(
            &self,
            other: &VirtualAddress,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            ::core::cmp::PartialOrd::partial_cmp(&self.0, &other.0)
        }
    }
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
            if (self.0 as isize) < 0 { TableKind::Kernel } else { TableKind::User }
        }
        pub fn as_str<'a>(&self) -> &'a str {
            self.0.numtoa_str(16, unsafe { &mut crate::GENERIC_STATIC_BUFFER })
        }
        pub unsafe fn as_ptr(&self) -> *mut u8 {
            core::mem::transmute::<_, *mut u8>(self.0)
        }
    }
    impl alloc::fmt::Debug for VirtualAddress {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.debug_tuple("VirtualAddress")
                .field_with(|v| v.write_fmt(format_args!("{0:#x}", &self.0)))
                .finish()
        }
    }
    pub struct MemoryArea {
        pub base: VirtualAddress,
        pub size: usize,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for MemoryArea {
        #[inline]
        fn clone(&self) -> MemoryArea {
            let _: ::core::clone::AssertParamIsClone<VirtualAddress>;
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for MemoryArea {}
    #[automatically_derived]
    impl ::core::fmt::Debug for MemoryArea {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "MemoryArea",
                "base",
                &self.base,
                "size",
                &&self.size,
            )
        }
    }
    impl MemoryArea {
        pub const fn new(base: usize, size: usize) -> Self {
            Self {
                base: VirtualAddress::new(base),
                size,
            }
        }
    }
}
pub mod debug {
    use core::{
        sync::atomic::{AtomicBool, Ordering::SeqCst},
        arch,
    };
    use crate::{serial::Port, device::character::UnsafeCharacterDevice};
    static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
    pub fn toggle_debug(boolean: bool) {
        DEBUG_ENABLED.store(boolean, SeqCst);
    }
    pub fn is_debug_enabled() -> bool {
        DEBUG_ENABLED.load(SeqCst)
    }
}
pub mod paging {
    use core::marker;
    use bit::BitIndex;
    use numtoa::NumToA;
    use crate::memory::PhysicalAddress;
    pub mod frame_allocator {
        use core::{ptr::NonNull, sync::atomic::AtomicPtr};
        use crate::{
            consts::{INVALID_PAGE_STATE, PAGE_SIZE},
            debug, endl, extra_features, memory::{MemoryArea, PhysicalAddress},
        };
        use alloc::sync::Arc;
        use limine::{MemmapEntry, MemmapResponse, MemoryMapEntryType, NonNullPtr};
        use x86_64::{
            structures::paging::{Size4KiB, PhysFrame},
            PhysAddr,
        };
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
        macro decl_multi_page_fn {
            ([$_meta : vis] $name : ident => $target_name : ident(...)) => { $_meta fn
            $name (& mut self, addr : usize, num : usize) -> Result < (), Error > { for i
            in 0..num { self.$target_name (addr + (i * crate ::consts::PAGE_SIZE as
            usize)) ?; } Ok(()) } }
        }
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
        #[automatically_derived]
        impl ::core::fmt::Debug for Error {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::write_str(
                    f,
                    match self {
                        Error::UninitializedAllocator => "UninitializedAllocator",
                        Error::OutOfMemory => "OutOfMemory",
                        Error::AlreadyDone => "AlreadyDone",
                        Error::CorruptedBitmap => "CorruptedBitmap",
                        Error::OutOfBitmapBounds => "OutOfBitmapBounds",
                        Error::InvalidPointer => "InvalidPointer",
                        Error::InvalidAlignment => "InvalidAlignment",
                        Error::Continue => "Continue",
                    },
                )
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralEq for Error {}
        #[automatically_derived]
        impl ::core::cmp::Eq for Error {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Error {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Error {
            #[inline]
            fn eq(&self, other: &Error) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        pub enum PageState {
            Reserved,
            Free,
            Used,
        }
        #[automatically_derived]
        impl ::core::marker::ConstParamTy for PageState {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for PageState {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for PageState {
            #[inline]
            fn eq(&self, other: &PageState) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralEq for PageState {}
        #[automatically_derived]
        impl ::core::cmp::Eq for PageState {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        impl PageFrameAllocator {
            pub unsafe fn from_response(resp: &MemmapResponse) -> PageFrameAllocator {
                if core::intrinsics::unlikely((resp).entry_count == 0) {
                    ::core::panicking::panic("not implemented")
                }
                let mut largest_free_segment = None;
                let mut largest_free_segment_size = 0;
                let mut total_memory = 0;
                for entry in (resp).memmap().iter() {
                    if entry.typ == MemoryMapEntryType::Usable
                        && entry.len > largest_free_segment_size
                    {
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
                    ::core::panicking::panic("not implemented")
                }
                static mut _BITMAP: core::mem::MaybeUninit<PageBitmap> = core::mem::MaybeUninit::uninit();
                if let Some(_seg) = segment {
                    *(unsafe {
                        &mut _BITMAP
                    }) = core::mem::MaybeUninit::<
                        PageBitmap,
                    >::new(
                        (unsafe {
                            PageBitmap::from_storage(
                                    pages + 1,
                                    (),
                                    PtrWrapper::<
                                        [usize],
                                    >::from_raw(
                                        core::slice::from_raw_parts_mut(
                                            _seg.base as *mut usize,
                                            pages + 1,
                                        ),
                                    ),
                                )
                                .unwrap()
                        }),
                    );
                    return unsafe { &mut _BITMAP };
                }
                ::core::panicking::panic("internal error: entered unreachable code");
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
                for fb in crate::FRAMEBUFFERS.iter() {
                    if let Some(ptr) = fb.address.as_ptr() {
                        self.lock_pages(
                            ptr.addr(),
                            (fb.height * fb.width / PAGE_SIZE) as usize + 1,
                        );
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
                    _ => {
                        ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        )
                    }
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
                    PageState::Free => ::core::panicking::panic("not implemented"),
                    PageState::Used => {
                        self.bitmap.set(index, 0);
                        self.used_memory -= PAGE_SIZE as usize;
                        self.free_memory += PAGE_SIZE as usize;
                        Ok(())
                    }
                    _ => {
                        ::core::panicking::panic(
                            "internal error: entered unreachable code",
                        )
                    }
                };
            }
            pub fn is_used_or_reserved(&mut self, addr: usize) -> bool {
                let index = addr / PAGE_SIZE as usize;
                return match self.bitmap.get(index).unwrap_or(INVALID_PAGE_STATE) {
                    0 => false,
                    1 => true,
                    _ => ::core::panicking::panic("not implemented"),
                };
            }
            pub fn free_page(&mut self, addr: usize) -> Result<(), Error> {
                let index: usize = (addr / crate::consts::PAGE_SIZE as usize);
                let state = self.bitmap.get(index).unwrap_or(INVALID_PAGE_STATE);
                return match state {
                    0 => Err(Error::AlreadyDone),
                    1 => {
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
                ::core::panicking::panic("internal error: entered unreachable code");
            }
            pub fn free_pages(&mut self, addr: usize, num: usize) -> Result<(), Error> {
                for i in 0..num {
                    self.free_page(addr + (i * crate::consts::PAGE_SIZE as usize))?;
                }
                Ok(())
            }
            pub fn lock_page(&mut self, addr: usize) -> Result<(), Error> {
                let index: usize = addr / crate::consts::PAGE_SIZE as usize;
                let state = self.bitmap.get(addr).unwrap_or(INVALID_PAGE_STATE);
                return match state {
                    0 => self.mark_page_as::<{ PageState::Used }>(index),
                    1 => Err(Error::AlreadyDone),
                    _ => Err(Error::OutOfBitmapBounds),
                };
                ::core::panicking::panic("internal error: entered unreachable code");
            }
            pub fn lock_pages(&mut self, addr: usize, num: usize) -> Result<(), Error> {
                for i in 0..num {
                    self.lock_page(addr + (i * crate::consts::PAGE_SIZE as usize))?;
                }
                Ok(())
            }
            pub fn request_page(&mut self) -> Result<usize, Error> {
                while self._bitmap_index < self.bitmap.len() * 8 {
                    {
                        let state = self
                            .bitmap
                            .get(self._bitmap_index)
                            .unwrap_or(INVALID_PAGE_STATE);
                        let matched_state = match state {
                            1 => Err(Error::Continue),
                            0 => {
                                self.mark_page_as::<
                                        { PageState::Used },
                                    >(self._bitmap_index)?;
                                return Ok(self._bitmap_index * PAGE_SIZE as usize);
                            }
                            _ => Err(Error::OutOfBitmapBounds),
                        };
                        if matched_state != Err(Error::Continue)
                            && matched_state.is_err()
                        {
                            return matched_state;
                        } else if matched_state.is_ok() {
                            return matched_state;
                        }
                    }
                    self._bitmap_index += 1;
                }
                Err(Error::OutOfMemory)
            }
            #[must_use]
            pub fn request_memory_area(
                &mut self,
                size: usize,
            ) -> Result<MemoryArea, Error> {
                let mut pages_left = (size / crate::consts::PAGE_SIZE as usize) + 1;
                let mut base: usize = 0;
                while self._bitmap_index < self.bitmap.len() * 8 && pages_left > 0 {
                    {
                        let state = self
                            .bitmap
                            .get(self._bitmap_index)
                            .unwrap_or(INVALID_PAGE_STATE);
                        match state {
                            1 => {
                                pages_left = size / crate::consts::PAGE_SIZE as usize;
                                base = 0;
                            }
                            0 => {
                                self.mark_page_as::<
                                        { PageState::Used },
                                    >(self._bitmap_index)?;
                                if base == 0 {
                                    base = self._bitmap_index * PAGE_SIZE as usize;
                                }
                                pages_left -= 1;
                            }
                            _ => return Err(Error::OutOfBitmapBounds),
                        };
                    }
                    self._bitmap_index += 1;
                }
                if base != 0 {
                    Ok(
                        MemoryArea::new(
                            PhysicalAddress::new(base).to_virtual().data(),
                            size,
                        ),
                    )
                } else {
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
            pub(self) fn reserve_pages(
                &mut self,
                addr: usize,
                num: usize,
            ) -> Result<(), Error> {
                for i in 0..num {
                    self.reserve_page(addr + (i * crate::consts::PAGE_SIZE as usize))?;
                }
                Ok(())
            }
            fn unreserve_page(&mut self, addr: usize) -> Result<(), Error> {
                let index: usize = (addr / crate::consts::PAGE_SIZE as usize);
                let state = self.bitmap.get(addr).unwrap_or(INVALID_PAGE_STATE);
                return match state {
                    1 => self.disable_page_mark::<{ PageState::Reserved }>(index),
                    0 => Err(Error::AlreadyDone),
                    _ => Err(Error::OutOfBitmapBounds),
                };
            }
            pub(self) fn unreserve_pages(
                &mut self,
                addr: usize,
                num: usize,
            ) -> Result<(), Error> {
                for i in 0..num {
                    self.unreserve_page(addr + (i * crate::consts::PAGE_SIZE as usize))?;
                }
                Ok(())
            }
            pub fn is_initialized(&self) -> bool {
                return self._initialized;
            }
            /// Safe version of `request_page`.
            pub fn request_safe_page<'a>(
                &mut self,
            ) -> Result<super::SafePagePtr, Error> {
                Ok(unsafe { super::SafePagePtr::unsafe_from_addr(self.request_page()?) })
            }
        }
        pub struct PtrWrapper<T: ?Sized> {
            pub(self) inner: NonNull<T>,
        }
        unsafe impl<T: ?Sized> Sync for PtrWrapper<T> {}
        unsafe impl<T: ?Sized> Send for PtrWrapper<T> {}
        impl<T: ?Sized> PtrWrapper<T> {
            pub unsafe fn from_raw(_val: &mut T) -> PtrWrapper<T> {
                Self {
                    inner: NonNull::new(_val).unwrap(),
                }
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
        unsafe impl x86_64::structures::paging::FrameAllocator<Size4KiB>
        for PageFrameAllocator {
            fn allocate_frame(
                &mut self,
            ) -> Option<x86_64::structures::paging::PhysFrame<Size4KiB>> {
                let page_base_addr = self.request_page().ok()?;
                PhysFrame::from_start_address(PhysAddr::new(page_base_addr as u64)).ok()
            }
        }
    }
    pub mod table_manager {
        use core::{char::UNICODE_VERSION, f64::consts::E, ptr::NonNull};
        use limine::NonNullPtr;
        use x86_64::structures::paging::page_table::PageTableEntry;
        use crate::{common::_alloc_frame_as_mut_t, debug, endl, assign_uninit};
        use super::{indexer::PageMapIndexer, PageTable};
        use crate::memory::{PhysicalAddress, VirtualAddress};
        pub enum MemoryMapError {
            FrameAllocator(super::frame_allocator::Error),
            InvalidAddress,
            TableNotFound,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for MemoryMapError {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    MemoryMapError::FrameAllocator(__self_0) => {
                        ::core::fmt::Formatter::debug_tuple_field1_finish(
                            f,
                            "FrameAllocator",
                            &__self_0,
                        )
                    }
                    MemoryMapError::InvalidAddress => {
                        ::core::fmt::Formatter::write_str(f, "InvalidAddress")
                    }
                    MemoryMapError::TableNotFound => {
                        ::core::fmt::Formatter::write_str(f, "TableNotFound")
                    }
                }
            }
        }
        #[allow(non_snake_case)]
        pub struct PageTableManager {
            PML4: NonNullPtr<PageTable>,
        }
        impl PageTableManager {
            pub fn new() -> Option<Self> {
                Some(PageTableManager {
                    PML4: unsafe {
                        core::mem::transmute(
                            NonNull::<
                                PageTable,
                            >::new(_alloc_frame_as_mut_t::<PageTable>().ok()?)?,
                        )
                    },
                })
            }
            pub fn from_pml4(pml4: NonNullPtr<PageTable>) -> Option<Self> {
                Some(PageTableManager { PML4: pml4 })
            }
            pub unsafe fn register(&self) -> &Self {
                self.map_memory(
                    VirtualAddress::new(self.PML4.as_ptr().addr()),
                    PhysicalAddress::new(self.PML4.as_ptr().addr()),
                );
                asm!("mov cr3, {0}", in (reg) self.PML4.as_ptr().addr());
                self
            }
            /// Internal Function for Mapping Memory.
            pub(crate) unsafe fn map_memory_internal(
                &self,
                virtual_addr: VirtualAddress,
                physical_addr: PhysicalAddress,
            ) -> Result<(), MemoryMapError> {
                let indexer = PageMapIndexer::for_addr(virtual_addr.data());
                unsafe {
                    crate::DEBUG_LINE
                        .unsafe_write_string("Not Mapped Virtual Address 0x");
                    crate::DEBUG_LINE.unsafe_write_string(virtual_addr.as_str());
                    crate::DEBUG_LINE.unsafe_write_string(" to 0x");
                    crate::DEBUG_LINE
                        .unsafe_write_string(
                            PhysicalAddress::new(physical_addr.data()).as_str(),
                        );
                    crate::DEBUG_LINE.unsafe_write_string("\n");
                };
                Ok(())
            }
            pub fn map_memory(
                &self,
                virtual_addr: VirtualAddress,
                physical_addr: PhysicalAddress,
            ) -> Result<(), MemoryMapError> {
                unsafe { self.map_memory_internal(virtual_addr, physical_addr) }
            }
        }
    }
    pub mod indexer {
        pub struct PageMapIndexer {
            pub pdp: usize,
            pub pd: usize,
            pub pt: usize,
            pub p: usize,
        }
        impl PageMapIndexer {
            pub fn for_addr(addr: usize) -> Self {
                let mut virtualAddress = addr;
                virtualAddress >>= 12;
                let p = virtualAddress & 0x1ff;
                virtualAddress >>= 9;
                let pt = virtualAddress & 0x1ff;
                virtualAddress >>= 9;
                let pd = virtualAddress & 0x1ff;
                virtualAddress >>= 9;
                let pdp = virtualAddress & 0x1ff;
                virtualAddress >>= 9;
                Self { pdp, pd, pt, p }
            }
        }
    }
    use spin::Mutex;
    use core::cell::UnsafeCell;
    /// Get a mutable reference to the [frame_allocator::PageFrameAllocator].
    /// This is now thread-safe and will not lead to undefined behavior.
    pub fn pf_allocator() -> spin::MutexGuard<
        'static,
        frame_allocator::PageFrameAllocator,
    > {
        crate::KERNEL_FRAME_ALLOCATOR.lock()
    }
    /// Get the Global [table_manager::PageTableManager].
    /// This is now thread-safe and will not lead to undefined behavior.
    pub fn pt_manager() -> spin::MutexGuard<'static, table_manager::PageTableManager> {
        crate::KERNEL_PAGE_TABLE_MANAGER.lock()
    }
    #[must_use]
    pub struct SafePagePtr(
        usize,
    )
    where
        Self: Sync + Sized;
    impl SafePagePtr {
        #[inline]
        pub fn new() -> Self {
            ::core::panicking::panic("not implemented");
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
            core::intrinsics::assume(
                core::mem::size_of::<T>() <= crate::consts::PAGE_SIZE as usize,
            );
            core::intrinsics::transmute_unchecked::<Self, *mut T>(self)
        }
        #[inline]
        #[must_use]
        pub fn free(&mut self) {
            pf_allocator().free_page(self.0).unwrap();
        }
    }
    impl Drop for SafePagePtr {
        fn drop(&mut self) {
            self.free();
        }
    }
    pub use x86_64::structures::paging::page_table::PageTableEntry;
    pub use x86_64::structures::paging::page_table::PageTable;
    use self::frame_allocator::PageFrameAllocator;
    pub struct PageFrameMapper;
}
pub mod renderer {
    pub type Color = u32;
    use core::ops::Add;
    use core::ptr::{addr_of, NonNull};
    use crate::bitmap_font::{BitmapFont, DisplayChar};
    use crate::common::endl;
    use crate::tty::KERNEL_CONSOLE;
    use crate::{debug, assign_uninit};
    use crate::framebuffer::Framebuffer;
    pub struct Renderer {
        target_fb: &'static Framebuffer,
        foreground_color: Color,
        background_color: Color,
        bitmap_font: &'static BitmapFont,
        pub optional_font_scaling: Option<u64>,
    }
    fn align_number_to(num: usize, align: usize) -> usize {
        num - (num % align)
    }
    fn mod_max(num: usize, max: usize) -> usize {
        if num > max { max } else { num }
    }
    pub enum RendererError {
        OutOfBounds,
    }
    static mut GLOBAL_RENDERER: core::mem::MaybeUninit<Renderer> = core::mem::MaybeUninit::uninit();
    impl Renderer {
        pub fn global<'a>() -> &'a Renderer {
            unsafe { &*self::GLOBAL_RENDERER.as_ptr() }
        }
        pub fn global_mut<'a>() -> &'a mut Renderer {
            unsafe { &mut *self::GLOBAL_RENDERER.as_mut_ptr() }
        }
        pub fn make_global(self) {
            *(unsafe {
                &mut GLOBAL_RENDERER
            }) = core::mem::MaybeUninit::<Renderer>::new((self));
        }
        pub fn new(fb: &'static Framebuffer, font: &'static BitmapFont) -> Renderer {
            Self {
                target_fb: fb,
                foreground_color: 0xFFFFFFFF,
                background_color: 0x00000000,
                bitmap_font: font,
                optional_font_scaling: None,
            }
        }
        pub fn get_font_scaling(&self) -> u64 {
            self.optional_font_scaling.unwrap_or(10)
        }
        pub unsafe fn scroll(&mut self, amount: usize, align: usize) {
            let mut base = self
                .target_fb
                .address
                .as_ptr()
                .map(|p| core::mem::transmute::<*mut u8, *mut Color>(p))
                .unwrap();
            let chunk_size = self.target_fb.width as usize * amount;
            let area_size = (self.target_fb.height * self.target_fb.width) as usize
                - chunk_size;
            let end_area = base.add(chunk_size);
            base.copy_from(base.add(chunk_size), area_size);
            Self::_fill_with_color(
                base.add(area_size),
                chunk_size,
                self.background_color,
                self.background_color,
            )
        }
        pub unsafe fn clear(&self, color: Color) {
            Self::_fill_with_color(
                self
                    .target_fb
                    .address
                    .as_ptr()
                    .map(|p| core::mem::transmute::<*mut u8, *mut Color>(p))
                    .unwrap(),
                (self.target_fb.height * self.target_fb.width) as usize,
                color,
                color,
            );
        }
        pub unsafe fn unsafe_put_pixel(&self, x: usize, y: usize, color: Color) {
            let mut pixel_offset = (mod_max(x, self.target_fb.width as usize) * 4)
                + self.target_fb.pitch as usize
                    * mod_max(y, self.target_fb.height as usize);
            unsafe {
                crate::DEBUG_LINE.unsafe_write_string("unsafe_put_pixel( x=");
                crate::DEBUG_LINE.unsafe_write_string(crate::integer_to_string(x));
                crate::DEBUG_LINE.unsafe_write_string(", y=");
                crate::DEBUG_LINE.unsafe_write_string(crate::integer_to_string(y));
                crate::DEBUG_LINE.unsafe_write_string(", offset =");
                crate::DEBUG_LINE
                    .unsafe_write_string(crate::integer_to_string(pixel_offset));
                crate::DEBUG_LINE.unsafe_write_string(" )");
                crate::DEBUG_LINE.unsafe_write_string("\n");
            };
            let mut pix = core::mem::transmute::<
                *mut u8,
                *mut Color,
            >(
                self
                    .target_fb
                    .address
                    .as_ptr()
                    .expect("Failed to get Pointer")
                    .offset(pixel_offset as isize),
            );
            pix.write(color)
        }
        pub unsafe fn unsafe_pull_pixel(&self, x: usize, y: usize) -> Color {
            let pixel_offset = (x * 4) + (self.target_fb.pitch as usize * y);
            *(self.target_fb.address.as_ptr().unwrap().offset(pixel_offset as isize)
                as *mut Color)
        }
        pub fn set_text_colors_via_invert(&mut self, color: Color) {
            self.foreground_color = color;
            self.background_color = !color;
        }
        pub fn update_colors(
            &mut self,
            fg_color: Option<Color>,
            bg_color: Option<Color>,
        ) {
            self.foreground_color = fg_color.unwrap_or(self.foreground_color);
            self.background_color = bg_color.unwrap_or(self.background_color);
        }
        pub unsafe fn _fill_with_color(
            base: *mut Color,
            amount: usize,
            filler: Color,
            background_color: Color,
        ) {
            for offset in 0..amount {
                let ptr = base.offset(offset as isize);
                let val = ptr.read();
                ptr.write(filler)
            }
        }
        pub unsafe fn unsafe_put_scaled_pixel(&self, x: usize, y: usize, color: Color) {
            let scaling = self.optional_font_scaling.unwrap_or(10) as usize;
            self.unsafe_fill_square(x * scaling, y * scaling, scaling, scaling, color);
        }
        pub unsafe fn unsafe_draw_line(
            &self,
            x0: usize,
            y0: usize,
            x1: usize,
            y1: usize,
            color: Color,
        ) {
            let dx = (x1 as isize - x0 as isize).abs();
            let dy = -(y1 as isize - y0 as isize).abs();
            let sx = if x0 < x1 { 1 as isize } else { -1 };
            let sy = if y0 < y1 { 1 as isize } else { -1 };
            let mut err = dx + dy;
            let mut x = x0 as isize;
            let mut y = y0 as isize;
            while x != (x1 as isize) || y != (y1 as isize) {
                self.unsafe_put_scaled_pixel(x as usize, y as usize, color);
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y += sy;
                }
            }
        }
        pub unsafe fn unsafe_fill_square(
            &self,
            x: usize,
            y: usize,
            w: usize,
            h: usize,
            color: Color,
        ) {
            for y_off in y..(y + h) {
                Self::_fill_with_color(
                    self
                        .target_fb
                        .address
                        .as_ptr()
                        .map(|p| core::mem::transmute::<*mut u8, *mut Color>(p))
                        .unwrap()
                        .offset(
                            ((x) + (self.target_fb.pitch as usize * (y_off) / 4))
                                as isize,
                        ),
                    w * 4,
                    color,
                    self.background_color,
                );
            }
        }
        pub fn dimensions(&self) -> (usize, usize) {
            (self.target_fb.width as usize, self.target_fb.height as usize)
        }
        pub unsafe fn unsafe_draw_char(
            &self,
            off_x: usize,
            off_y: usize,
            chr: u8,
        ) -> usize {
            let scaling = self.optional_font_scaling.unwrap_or(10) as usize;
            let line_off = 0;
            for x in 0..8 as usize {
                for y in 0..8 as usize {
                    self.unsafe_fill_square(
                        off_x + (x * scaling),
                        off_y + (y * scaling),
                        scaling,
                        scaling,
                        if self.bitmap_font[chr as usize].is_set(x, y) {
                            self.foreground_color
                        } else {
                            self.background_color
                        },
                    );
                }
            }
            line_off
        }
        pub unsafe fn draw_raw_image(&self, x: usize, y: usize, pixels: &'_ [u8]) {
            self.target_fb
                .address
                .as_ptr()
                .unwrap()
                .offset((x + (self.target_fb.pitch as usize * (y))) as isize)
                .copy_from(pixels.as_ptr(), pixels.len());
        }
        pub unsafe fn unsafe_draw_text(
            &self,
            x: usize,
            y: usize,
            text: &'_ str,
        ) -> usize {
            let scaling = self.optional_font_scaling.unwrap_or(10) as usize;
            let mut line_off = 0usize;
            for (index, chr) in text.chars().enumerate() {
                if chr == '\n' {
                    line_off += 1;
                    continue;
                }
                let x_off = x + (index * (16 * scaling));
                line_off += self.unsafe_draw_char(x_off, y + (line_off * 16), chr as u8);
            }
            line_off
        }
    }
}
pub mod serial {
    #[macro_use]
    use crate::common::macros;
    use crate::common::io::{inb, io_wait, outb};
    use crate::device::{
        character::{CharacterDeviceMode, TimedCharacterDevice, UnsafeCharacterDevice},
        Device, GeneralDevice,
    };
    use crate::kprint;
    pub enum Port {
        COM1 = 0x3F8,
        COM2 = 0x2F8,
        COM3 = 0x3E8,
        COM4 = 0x2E8,
        COM5 = 0x5F8,
        COM6 = 0x4F8,
        COM7 = 0x5E8,
        COM8 = 0x4E8,
    }
    impl Port {
        pub fn get_addr(&self) -> u16 {
            match self {
                Port::COM1 => 0x3F8,
                Port::COM2 => 0x2F8,
                Port::COM3 => 0x3E8,
                Port::COM4 => 0x2E8,
                Port::COM5 => 0x5F8,
                Port::COM6 => 0x4F8,
                Port::COM7 => 0x5E8,
                Port::COM8 => 0x4E8,
            }
        }
    }
    impl GeneralDevice for Port {
        fn as_device(&self) -> crate::device::Device<'_> {
            Device::Character(self)
        }
    }
    impl UnsafeCharacterDevice for Port {
        unsafe fn read_raw(&self) -> u8 {
            inb(self.get_addr()) as u8
        }
        unsafe fn write_raw(&self, data: u8) {
            outb(self.get_addr(), data as u8);
        }
        unsafe fn test(&self) -> bool {
            outb(self.get_addr() + 1, 0x00);
            outb(self.get_addr() + 3, 0x80);
            outb(self.get_addr() + 0, 0x03);
            outb(self.get_addr() + 1, 0x00);
            outb(self.get_addr() + 3, 0x03);
            outb(self.get_addr() + 2, 0xC7);
            outb(self.get_addr() + 4, 0x0B);
            outb(self.get_addr() + 4, 0x1E);
            outb(self.get_addr() + 0, 0xAE);
            if (inb(self.get_addr() + 0) != 0xAE) {
                return true;
            }
            outb(self.get_addr() + 4, 0x0F);
            return false;
        }
        unsafe fn init(&mut self) -> bool {
            true
        }
        unsafe fn received(&self) -> bool {
            (inb(self.get_addr() + 5) & 1) != 0
        }
        unsafe fn is_transmit_empty(&self) -> bool {
            (inb(self.get_addr() + 5) & 0x20) != 0
        }
        fn set_mode(&mut self, mode: CharacterDeviceMode) {}
        fn get_mode(&self) -> CharacterDeviceMode {
            CharacterDeviceMode::Normal
        }
    }
    impl TimedCharacterDevice for Port {
        unsafe fn read(&self) -> u8 {
            while !self.received() {}
            self.read_raw()
        }
        unsafe fn write(&self, data: u8) {
            while !self.is_transmit_empty() {}
            self.write_raw(data)
        }
        unsafe fn wait(&self) {}
    }
    impl Port
    where
        Self: UnsafeCharacterDevice,
        Self: TimedCharacterDevice,
    {
        pub fn wait_for_connection(&self) {
            unsafe {
                while self.test() {}
                unsafe {
                    crate::KERNEL_CONSOLE
                        .write_str("Successfully connected to serial line.");
                    crate::KERNEL_CONSOLE.newline();
                }
            }
        }
        pub unsafe fn unsafe_write_string(&self, _str: &'_ str) {
            for chr in _str.chars() {
                self.write(chr as u8);
            }
        }
        pub unsafe fn unsafe_write_line(&self, _str: &'_ str) {
            self.unsafe_write_string(_str);
            self.unsafe_write_string("\n\r");
        }
        pub unsafe fn unsafe_read_string(&self, len: usize) -> &'static str {
            ::core::panicking::panic("not implemented");
        }
    }
}
pub mod status {}
pub mod tty {
    use alloc::format;
    use crate::{
        device::{Device, GeneralDevice, character::UnsafeCharacterDevice},
        common::endl, renderer::{self, Color, Renderer},
    };
    pub struct Console {
        pub cursor_pos: (usize, usize),
        pub line_padding: usize,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Console {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "Console",
                "cursor_pos",
                &self.cursor_pos,
                "line_padding",
                &&self.line_padding,
            )
        }
    }
    #[automatically_derived]
    impl ::core::default::Default for Console {
        #[inline]
        fn default() -> Console {
            Console {
                cursor_pos: ::core::default::Default::default(),
                line_padding: ::core::default::Default::default(),
            }
        }
    }
    impl GeneralDevice for Console {
        fn as_device(&self) -> Device<'_> {
            Device::Character(self)
        }
    }
    impl UnsafeCharacterDevice for Console {
        unsafe fn read_raw(&self) -> u8 {
            ::core::panicking::panic("not implemented")
        }
        unsafe fn write_raw(&self, data: u8) {
            let _ = super::renderer::Renderer::global()
                .unsafe_draw_char(
                    self.cursor_pos.0 * 16,
                    self.cursor_pos.1 * (16 + self.line_padding),
                    data,
                );
        }
        unsafe fn received(&self) -> bool {
            false
        }
        unsafe fn is_transmit_empty(&self) -> bool {
            true
        }
        unsafe fn test(&self) -> bool {
            true
        }
        unsafe fn init(&mut self) -> bool {
            ::core::panicking::panic("not yet implemented")
        }
        fn set_mode(&mut self, mode: crate::device::character::CharacterDeviceMode) {}
        fn get_mode(&self) -> crate::device::character::CharacterDeviceMode {
            crate::device::character::CharacterDeviceMode::Normal
        }
    }
    pub(crate) static mut KERNEL_CONSOLE: Console = Console::new();
    pub const COLORS: [Color; 8] = [
        0x00000000,
        0xFFFFFFFF,
        0xFFFF0000,
        0xFF00FF00,
        0xFF0000FF,
        0xFFFFFF00,
        0xFFFF00FF,
        0xFF00FFFF,
    ];
    impl Console
    where
        Self: UnsafeCharacterDevice,
    {
        pub fn write_str(&mut self, _str: &'_ str) {
            for (idx, chr) in _str.chars().enumerate() {
                if self.cursor_pos.0 > super::Renderer::global().dimensions().0 / 8 {
                    self.newline()
                }
                if self.cursor_pos.1 > super::Renderer::global().dimensions().1 {
                    self.scroll();
                }
                if chr == '\n' {
                    self.newline();
                    continue;
                }
                unsafe { self.write_raw(chr as u8) }
                self.cursor_pos.0 += 1;
            }
        }
        pub fn get_line_padding(&mut self) -> usize {
            16 + self.line_padding
        }
        pub fn print(&mut self, _str: &'_ str) {
            self.write_str(_str);
            self.newline();
        }
        pub fn newline(&mut self) {
            self.cursor_pos.1 += 1;
            self.cursor_pos.0 = 0;
        }
        pub fn scroll(&mut self) {
            unsafe { Renderer::global_mut().scroll(8, 2) };
            self.cursor_pos.1 -= 1;
        }
        pub const fn new() -> Self {
            Self {
                cursor_pos: (0, 0),
                line_padding: 0,
            }
        }
    }
}
use alloc::format;
use alloc_impl as _;
use paging::frame_allocator::PageFrameAllocator;
use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::{Mapper, PageTableFlags, PhysFrame, Size2MiB, Size4KiB};
use x86_64::{structures, PhysAddr, VirtAddr};
use crate::alloc_impl::KERNEL_ALLOCATOR;
use crate::common::idt::{Idt, KERNEL_IDT};
use crate::common::io::outb;
use crate::common::*;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use crate::memory::{
    active_level_4_table, PhysicalAddress, VirtualAddress,
    PHYSICAL_BOOTLOADER_MEMORY_OFFSET, PHYSICAL_MEMORY_OFFSET,
};
use crate::paging::table_manager::PageTableManager;
use core::arch::asm;
use core::ffi::CStr;
use elf::endian::AnyEndian;
use elf::segment::ProgramHeader;
use memory::MemoryArea;
pub(crate) use tty::KERNEL_CONSOLE;
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use crate::paging::{pf_allocator, pt_manager, PageTable};
use core::mem;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
#[macro_use]
extern crate bitfield;
use numtoa::NumToA;
use renderer::Renderer;
#[macro_use]
extern crate antos_macros;
static FRAMEBUFFERS_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(
    0,
);
static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static KERNEL_ADDRESS_REQUEST: limine::KernelAddressRequest = limine::KernelAddressRequest::new(
    0,
);
static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(
    0,
);
static MODULE_REQUEST: limine::ModuleRequest = limine::ModuleRequest::new(0);
static mut SYSTEM_IDT: structures::idt::InterruptDescriptorTable = structures::idt::InterruptDescriptorTable::new();
pub(crate) static mut GENERIC_STATIC_BUFFER: [u8; 25] = [0u8; 25];
static TEST1: &'static str = "Hello Paging!";
static TEST2: &'static str = "):";
static mut ITOA_BUFFER: core::mem::MaybeUninit<itoa::Buffer> = core::mem::MaybeUninit::uninit();
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub(crate) struct FRAMEBUFFERS {
    __private_field: (),
}
#[doc(hidden)]
pub(crate) static FRAMEBUFFERS: FRAMEBUFFERS = FRAMEBUFFERS {
    __private_field: (),
};
impl ::lazy_static::__Deref for FRAMEBUFFERS {
    type Target = &'static [NonNullPtr<limine::Framebuffer>];
    fn deref(&self) -> &&'static [NonNullPtr<limine::Framebuffer>] {
        #[inline(always)]
        fn __static_ref_initialize() -> &'static [NonNullPtr<limine::Framebuffer>] {
            {
                if let Some(fb_resp) = FRAMEBUFFERS_REQUEST.get_response().get() {
                    unsafe { core::mem::transmute(fb_resp.framebuffers()) }
                } else {
                    unsafe {
                        crate::DEBUG_LINE.unsafe_write_string("ERROR: ");
                        crate::DEBUG_LINE
                            .unsafe_write_string(
                                "Failed to get the list of System Framebuffers!",
                            );
                    };
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Failed to get the list of System Framebuffers!",
                            ),
                        );
                    };
                }
            }
        }
        #[inline(always)]
        fn __stability() -> &'static &'static [NonNullPtr<limine::Framebuffer>] {
            static LAZY: ::lazy_static::lazy::Lazy<
                &'static [NonNullPtr<limine::Framebuffer>],
            > = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for FRAMEBUFFERS {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub(crate) struct KERNEL_BASE {
    __private_field: (),
}
#[doc(hidden)]
pub(crate) static KERNEL_BASE: KERNEL_BASE = KERNEL_BASE { __private_field: () };
impl ::lazy_static::__Deref for KERNEL_BASE {
    type Target = &'static limine::KernelAddressResponse;
    fn deref(&self) -> &&'static limine::KernelAddressResponse {
        #[inline(always)]
        fn __static_ref_initialize() -> &'static limine::KernelAddressResponse {
            {
                if let Some(resp) = KERNEL_ADDRESS_REQUEST
                    .get_response()
                    .get::<'static>()
                {
                    resp
                } else {
                    unsafe {
                        crate::DEBUG_LINE.unsafe_write_string("ERROR: ");
                        crate::DEBUG_LINE
                            .unsafe_write_string(
                                "Failed to get the list of System Framebuffers!",
                            );
                    };
                    {
                        let lvl = ::log::Level::Error;
                        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                            ::log::__private_api::log(
                                format_args!(
                                    "Failed to get the list of System Framebuffers!",
                                ),
                                lvl,
                                &(
                                    "antos_kernel_minimal_generic",
                                    "antos_kernel_minimal_generic",
                                    "src/main.rs",
                                ),
                                124u32,
                                ::log::__private_api::Option::None,
                            );
                        }
                    };
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Failed to get the list of System Framebuffers!",
                            ),
                        );
                    };
                }
            }
        }
        #[inline(always)]
        fn __stability() -> &'static &'static limine::KernelAddressResponse {
            static LAZY: ::lazy_static::lazy::Lazy<
                &'static limine::KernelAddressResponse,
            > = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_BASE {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct KERNEL_FRAME_ALLOCATOR {
    __private_field: (),
}
#[doc(hidden)]
static KERNEL_FRAME_ALLOCATOR: KERNEL_FRAME_ALLOCATOR = KERNEL_FRAME_ALLOCATOR {
    __private_field: (),
};
impl ::lazy_static::__Deref for KERNEL_FRAME_ALLOCATOR {
    type Target = Mutex<PageFrameAllocator>;
    fn deref(&self) -> &Mutex<PageFrameAllocator> {
        #[inline(always)]
        fn __static_ref_initialize() -> Mutex<PageFrameAllocator> {
            Mutex::new(unsafe { PageFrameAllocator::from_response(&*KERNEL_MEMMAP) })
        }
        #[inline(always)]
        fn __stability() -> &'static Mutex<PageFrameAllocator> {
            static LAZY: ::lazy_static::lazy::Lazy<Mutex<PageFrameAllocator>> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_FRAME_ALLOCATOR {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct KERNEL_PAGE_TABLE_MANAGER {
    __private_field: (),
}
#[doc(hidden)]
static KERNEL_PAGE_TABLE_MANAGER: KERNEL_PAGE_TABLE_MANAGER = KERNEL_PAGE_TABLE_MANAGER {
    __private_field: (),
};
impl ::lazy_static::__Deref for KERNEL_PAGE_TABLE_MANAGER {
    type Target = Mutex<PageTableManager>;
    fn deref(&self) -> &Mutex<PageTableManager> {
        #[inline(always)]
        fn __static_ref_initialize() -> Mutex<PageTableManager> {
            Mutex::new(
                PageTableManager::new().expect("Failed to create Page Table Manager."),
            )
        }
        #[inline(always)]
        fn __stability() -> &'static Mutex<PageTableManager> {
            static LAZY: ::lazy_static::lazy::Lazy<Mutex<PageTableManager>> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_PAGE_TABLE_MANAGER {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct KERNEL_PAGE_MAPPER {
    __private_field: (),
}
#[doc(hidden)]
static KERNEL_PAGE_MAPPER: KERNEL_PAGE_MAPPER = KERNEL_PAGE_MAPPER {
    __private_field: (),
};
impl ::lazy_static::__Deref for KERNEL_PAGE_MAPPER {
    type Target = Mutex<x86_64::structures::paging::OffsetPageTable<'static>>;
    fn deref(&self) -> &Mutex<x86_64::structures::paging::OffsetPageTable<'static>> {
        #[inline(always)]
        fn __static_ref_initialize() -> Mutex<
            x86_64::structures::paging::OffsetPageTable<'static>,
        > {
            Mutex::new(unsafe {
                x86_64::structures::paging::mapper::OffsetPageTable::new(
                    unsafe { active_level_4_table(VirtAddr::zero()) },
                    VirtAddr::zero(),
                )
            })
        }
        #[inline(always)]
        fn __stability() -> &'static Mutex<
            x86_64::structures::paging::OffsetPageTable<'static>,
        > {
            static LAZY: ::lazy_static::lazy::Lazy<
                Mutex<x86_64::structures::paging::OffsetPageTable<'static>>,
            > = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_PAGE_MAPPER {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub(crate) struct KERNEL_FILE {
    __private_field: (),
}
#[doc(hidden)]
pub(crate) static KERNEL_FILE: KERNEL_FILE = KERNEL_FILE { __private_field: () };
impl ::lazy_static::__Deref for KERNEL_FILE {
    type Target = &'static limine::File;
    fn deref(&self) -> &&'static limine::File {
        #[inline(always)]
        fn __static_ref_initialize() -> &'static limine::File {
            {
                if let Some(resp) = KERNEL_FILE_REQUEST.get_response().get() {
                    resp.kernel_file.get::<'static>().unwrap()
                } else {
                    unsafe {
                        crate::DEBUG_LINE.unsafe_write_string("ERROR: ");
                        crate::DEBUG_LINE
                            .unsafe_write_string(
                                "Failed to get the list of System Framebuffers!",
                            );
                    };
                    {
                        let lvl = ::log::Level::Error;
                        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                            ::log::__private_api::log(
                                format_args!(
                                    "Failed to get the list of System Framebuffers!",
                                ),
                                lvl,
                                &(
                                    "antos_kernel_minimal_generic",
                                    "antos_kernel_minimal_generic",
                                    "src/main.rs",
                                ),
                                136u32,
                                ::log::__private_api::Option::None,
                            );
                        }
                    };
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Failed to get the list of System Framebuffers!",
                            ),
                        );
                    };
                }
            }
        }
        #[inline(always)]
        fn __stability() -> &'static &'static limine::File {
            static LAZY: ::lazy_static::lazy::Lazy<&'static limine::File> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_FILE {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
///The Area of Memory the Kernel Uses.
struct KERNEL_AREA {
    __private_field: (),
}
#[doc(hidden)]
static KERNEL_AREA: KERNEL_AREA = KERNEL_AREA { __private_field: () };
impl ::lazy_static::__Deref for KERNEL_AREA {
    type Target = MemoryArea;
    fn deref(&self) -> &MemoryArea {
        #[inline(always)]
        fn __static_ref_initialize() -> MemoryArea {
            MemoryArea::new(
                KERNEL_BASE.virtual_base as usize,
                KERNEL_FILE.length as usize,
            )
        }
        #[inline(always)]
        fn __stability() -> &'static MemoryArea {
            static LAZY: ::lazy_static::lazy::Lazy<MemoryArea> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_AREA {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct L4_PAGE_TABLE {
    __private_field: (),
}
#[doc(hidden)]
static L4_PAGE_TABLE: L4_PAGE_TABLE = L4_PAGE_TABLE {
    __private_field: (),
};
impl ::lazy_static::__Deref for L4_PAGE_TABLE {
    type Target = &'static mut PageTable;
    fn deref(&self) -> &&'static mut PageTable {
        #[inline(always)]
        fn __static_ref_initialize() -> &'static mut PageTable {
            unsafe { active_level_4_table(VirtAddr::zero()) }
        }
        #[inline(always)]
        fn __stability() -> &'static &'static mut PageTable {
            static LAZY: ::lazy_static::lazy::Lazy<&'static mut PageTable> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for L4_PAGE_TABLE {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
#[allow(missing_copy_implementations)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
struct KERNEL_MEMMAP {
    __private_field: (),
}
#[doc(hidden)]
static KERNEL_MEMMAP: KERNEL_MEMMAP = KERNEL_MEMMAP {
    __private_field: (),
};
impl ::lazy_static::__Deref for KERNEL_MEMMAP {
    type Target = &'static limine::MemmapResponse;
    fn deref(&self) -> &&'static limine::MemmapResponse {
        #[inline(always)]
        fn __static_ref_initialize() -> &'static limine::MemmapResponse {
            {
                if let Some(resp) = MEMMAP_REQUEST.get_response().get() {
                    resp
                } else {
                    unsafe {
                        crate::DEBUG_LINE.unsafe_write_string("ERROR: ");
                        crate::DEBUG_LINE
                            .unsafe_write_string(
                                "Failed to get the list of System Framebuffers!",
                            );
                    };
                    {
                        let lvl = ::log::Level::Error;
                        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                            ::log::__private_api::log(
                                format_args!(
                                    "Failed to get the list of System Framebuffers!",
                                ),
                                lvl,
                                &(
                                    "antos_kernel_minimal_generic",
                                    "antos_kernel_minimal_generic",
                                    "src/main.rs",
                                ),
                                148u32,
                                ::log::__private_api::Option::None,
                            );
                        }
                    };
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Failed to get the list of System Framebuffers!",
                            ),
                        );
                    };
                }
            }
        }
        #[inline(always)]
        fn __stability() -> &'static &'static limine::MemmapResponse {
            static LAZY: ::lazy_static::lazy::Lazy<&'static limine::MemmapResponse> = ::lazy_static::lazy::Lazy::INIT;
            LAZY.get(__static_ref_initialize)
        }
        __stability()
    }
}
impl ::lazy_static::LazyStatic for KERNEL_MEMMAP {
    fn initialize(lazy: &Self) {
        let _ = &**lazy;
    }
}
enum OutlineSegment {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Stop,
}
#[automatically_derived]
impl ::core::fmt::Debug for OutlineSegment {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            OutlineSegment::MoveTo(__self_0, __self_1) => {
                ::core::fmt::Formatter::debug_tuple_field2_finish(
                    f,
                    "MoveTo",
                    __self_0,
                    &__self_1,
                )
            }
            OutlineSegment::LineTo(__self_0, __self_1) => {
                ::core::fmt::Formatter::debug_tuple_field2_finish(
                    f,
                    "LineTo",
                    __self_0,
                    &__self_1,
                )
            }
            OutlineSegment::QuadTo(__self_0, __self_1, __self_2, __self_3) => {
                ::core::fmt::Formatter::debug_tuple_field4_finish(
                    f,
                    "QuadTo",
                    __self_0,
                    __self_1,
                    __self_2,
                    &__self_3,
                )
            }
            OutlineSegment::CurveTo(
                __self_0,
                __self_1,
                __self_2,
                __self_3,
                __self_4,
                __self_5,
            ) => {
                let values: &[&dyn ::core::fmt::Debug] = &[
                    __self_0,
                    __self_1,
                    __self_2,
                    __self_3,
                    __self_4,
                    &__self_5,
                ];
                ::core::fmt::Formatter::debug_tuple_fields_finish(f, "CurveTo", values)
            }
            OutlineSegment::Stop => ::core::fmt::Formatter::write_str(f, "Stop"),
        }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for OutlineSegment {
    #[inline]
    fn clone(&self) -> OutlineSegment {
        let _: ::core::clone::AssertParamIsClone<f32>;
        *self
    }
}
#[automatically_derived]
impl ::core::marker::Copy for OutlineSegment {}
type HugePage = x86_64::structures::paging::Page<Size2MiB>;
#[no_mangle]
unsafe extern "C" fn __prog_debug_print(__base: *const u8, __len: usize) {
    KERNEL_CONSOLE
        .write_str(
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(__base, __len)),
        );
}
type DbgPrintFn = unsafe extern "C" fn(*const u8, usize);
extern "x86-interrupt" fn handle_pit(_frame: InterruptStackFrame) {
    unsafe {
        crate::DEBUG_LINE
            .unsafe_write_string(
                {
                    let res = ::alloc::fmt::format(format_args!("Tick Tock"));
                    res
                }
                    .as_str(),
            )
    }
}
#[repr(C)]
pub struct KernelProgramMeta {
    _dbg_print: *const DbgPrintFn,
}
const MAX_FONT_OUTLINE_SEGMENTS: usize = 25;
pub struct FontOutline(heapless::Vec<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS>);
impl FontOutline {
    pub const fn new() -> Self {
        Self(heapless::Vec::<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS>::new())
    }
    pub const fn segments(
        &self,
    ) -> &'_ heapless::Vec<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS> {
        &(self.0)
    }
    pub fn push(&mut self, seg: OutlineSegment) {
        self.0.push(seg).expect("Failed to push Font Segment");
    }
}
impl ttf_parser::OutlineBuilder for FontOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push(OutlineSegment::MoveTo(x, y))
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(OutlineSegment::LineTo(x, y))
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.push(OutlineSegment::QuadTo(x1, y1, x, y))
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.push(OutlineSegment::CurveTo(x1, y1, x2, y2, x, y))
    }
    fn close(&mut self) {
        self.push(OutlineSegment::Stop)
    }
}
pub const FONT_BITMAP: bitmap_font::BitmapFont = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00],
    [0x36, 0x36, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00],
    [0x0C, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x0C, 0x00],
    [0x00, 0x63, 0x33, 0x18, 0x0C, 0x66, 0x63, 0x00],
    [0x1C, 0x36, 0x1C, 0x6E, 0x3B, 0x33, 0x6E, 0x00],
    [0x06, 0x06, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x18, 0x0C, 0x06, 0x06, 0x06, 0x0C, 0x18, 0x00],
    [0x06, 0x0C, 0x18, 0x18, 0x18, 0x0C, 0x06, 0x00],
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
    [0x00, 0x0C, 0x0C, 0x3F, 0x0C, 0x0C, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x00, 0x00, 0x00, 0x3F, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x60, 0x30, 0x18, 0x0C, 0x06, 0x03, 0x01, 0x00],
    [0x3E, 0x63, 0x73, 0x7B, 0x6F, 0x67, 0x3E, 0x00],
    [0x0C, 0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x06, 0x33, 0x3F, 0x00],
    [0x1E, 0x33, 0x30, 0x1C, 0x30, 0x33, 0x1E, 0x00],
    [0x38, 0x3C, 0x36, 0x33, 0x7F, 0x30, 0x78, 0x00],
    [0x3F, 0x03, 0x1F, 0x30, 0x30, 0x33, 0x1E, 0x00],
    [0x1C, 0x06, 0x03, 0x1F, 0x33, 0x33, 0x1E, 0x00],
    [0x3F, 0x33, 0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x00],
    [0x1E, 0x33, 0x33, 0x1E, 0x33, 0x33, 0x1E, 0x00],
    [0x1E, 0x33, 0x33, 0x3E, 0x30, 0x18, 0x0E, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x00],
    [0x00, 0x0C, 0x0C, 0x00, 0x00, 0x0C, 0x0C, 0x06],
    [0x18, 0x0C, 0x06, 0x03, 0x06, 0x0C, 0x18, 0x00],
    [0x00, 0x00, 0x3F, 0x00, 0x00, 0x3F, 0x00, 0x00],
    [0x06, 0x0C, 0x18, 0x30, 0x18, 0x0C, 0x06, 0x00],
    [0x1E, 0x33, 0x30, 0x18, 0x0C, 0x00, 0x0C, 0x00],
    [0x3E, 0x63, 0x7B, 0x7B, 0x7B, 0x03, 0x1E, 0x00],
    [0x0C, 0x1E, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x66, 0x66, 0x3F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x03, 0x66, 0x3C, 0x00],
    [0x1F, 0x36, 0x66, 0x66, 0x66, 0x36, 0x1F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x46, 0x7F, 0x00],
    [0x7F, 0x46, 0x16, 0x1E, 0x16, 0x06, 0x0F, 0x00],
    [0x3C, 0x66, 0x03, 0x03, 0x73, 0x66, 0x7C, 0x00],
    [0x33, 0x33, 0x33, 0x3F, 0x33, 0x33, 0x33, 0x00],
    [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x78, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E, 0x00],
    [0x67, 0x66, 0x36, 0x1E, 0x36, 0x66, 0x67, 0x00],
    [0x0F, 0x06, 0x06, 0x06, 0x46, 0x66, 0x7F, 0x00],
    [0x63, 0x77, 0x7F, 0x7F, 0x6B, 0x63, 0x63, 0x00],
    [0x63, 0x67, 0x6F, 0x7B, 0x73, 0x63, 0x63, 0x00],
    [0x1C, 0x36, 0x63, 0x63, 0x63, 0x36, 0x1C, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x06, 0x06, 0x0F, 0x00],
    [0x1E, 0x33, 0x33, 0x33, 0x3B, 0x1E, 0x38, 0x00],
    [0x3F, 0x66, 0x66, 0x3E, 0x36, 0x66, 0x67, 0x00],
    [0x1E, 0x33, 0x07, 0x0E, 0x38, 0x33, 0x1E, 0x00],
    [0x3F, 0x2D, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x3F, 0x00],
    [0x33, 0x33, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
    [0x63, 0x63, 0x36, 0x1C, 0x1C, 0x36, 0x63, 0x00],
    [0x33, 0x33, 0x33, 0x1E, 0x0C, 0x0C, 0x1E, 0x00],
    [0x7F, 0x63, 0x31, 0x18, 0x4C, 0x66, 0x7F, 0x00],
    [0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x1E, 0x00],
    [0x03, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
    [0x1E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x1E, 0x00],
    [0x08, 0x1C, 0x36, 0x63, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF],
    [0x0C, 0x0C, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x1E, 0x30, 0x3E, 0x33, 0x6E, 0x00],
    [0x07, 0x06, 0x06, 0x3E, 0x66, 0x66, 0x3B, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x03, 0x33, 0x1E, 0x00],
    [0x38, 0x30, 0x30, 0x3e, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x3f, 0x03, 0x1E, 0x00],
    [0x1C, 0x36, 0x06, 0x0f, 0x06, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x07, 0x06, 0x36, 0x6E, 0x66, 0x66, 0x67, 0x00],
    [0x0C, 0x00, 0x0E, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x30, 0x00, 0x30, 0x30, 0x30, 0x33, 0x33, 0x1E],
    [0x07, 0x06, 0x66, 0x36, 0x1E, 0x36, 0x67, 0x00],
    [0x0E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00],
    [0x00, 0x00, 0x33, 0x7F, 0x7F, 0x6B, 0x63, 0x00],
    [0x00, 0x00, 0x1F, 0x33, 0x33, 0x33, 0x33, 0x00],
    [0x00, 0x00, 0x1E, 0x33, 0x33, 0x33, 0x1E, 0x00],
    [0x00, 0x00, 0x3B, 0x66, 0x66, 0x3E, 0x06, 0x0F],
    [0x00, 0x00, 0x6E, 0x33, 0x33, 0x3E, 0x30, 0x78],
    [0x00, 0x00, 0x3B, 0x6E, 0x66, 0x06, 0x0F, 0x00],
    [0x00, 0x00, 0x3E, 0x03, 0x1E, 0x30, 0x1F, 0x00],
    [0x08, 0x0C, 0x3E, 0x0C, 0x0C, 0x2C, 0x18, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x33, 0x6E, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x1E, 0x0C, 0x00],
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x7F, 0x36, 0x00],
    [0x00, 0x00, 0x63, 0x36, 0x1C, 0x36, 0x63, 0x00],
    [0x00, 0x00, 0x33, 0x33, 0x33, 0x3E, 0x30, 0x1F],
    [0x00, 0x00, 0x3F, 0x19, 0x0C, 0x26, 0x3F, 0x00],
    [0x38, 0x0C, 0x0C, 0x07, 0x0C, 0x0C, 0x38, 0x00],
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00],
    [0x07, 0x0C, 0x0C, 0x38, 0x0C, 0x0C, 0x07, 0x00],
    [0x6E, 0x3B, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
];
pub const DEBUG_LINE: serial::Port = serial::Port::COM1;
pub fn integer_to_string<'_str, I: itoa::Integer>(value: I) -> &'_str str {
    let mut buf = unsafe { &mut ITOA_BUFFER };
    *(unsafe {
        &mut ITOA_BUFFER
    }) = core::mem::MaybeUninit::<itoa::Buffer>::new(({ itoa::Buffer::new() }));
    unsafe { (*(*buf).as_mut_ptr()).format::<I>(value) }
}
struct Person {
    id: i32,
    age: i32,
    name: &'static str,
}
#[repr(C)]
pub struct RegisterCapture {
    pub rax: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rbp: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}
pub static mut INTERRUPT_HANDLERS: [Option<
    unsafe fn(InterruptStackFrame, RegisterCapture),
>; 255] = [None; 255];
/// Captures the Registers of the CPU.
pub macro capture_registers {
    () => { { let mut rax = 0; let mut rbx = 0; let mut rcx = 0; let mut rdx = 0; let mut
    rsi = 0; let mut rdi = 0; let mut rbp = 0; let mut rsp = 0; let mut r8 = 0; let mut
    r9 = 0; let mut r10 = 0; let mut r11 = 0; let mut r12 = 0; let mut r13 = 0; let mut
    r14 = 0; let mut r15 = 0; ::core::arch::asm!("#CAPTURE_REGISTERS", out("rax") rax,
    out("rcx") rcx, out("rdx") rdx, out("rsi") rsi, out("rdi") rdi, out("r8") r8,
    out("r9") r9, out("r10") r10, out("r11") r11, out("r12") r12, out("r13") r13,
    out("r14") r14, out("r15") r15, options(nostack, nomem, preserves_flags));
    RegisterCapture { rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp, r8, r9, r10, r11, r12, r13,
    r14, r15, } } }
}
/// Apply Registers and Return from Interrupt.
/// ====================================
///
/// # Arguments
///
/// * `registers` - The Registers to apply.
/// * `capture` - The Capture to apply the Registers from.
/// * `frame` - The Interrupt Stack Frame.
///
/// # Safety
/// This macro is unsafe because it uses inline assembly.
///
/// See [`InterruptStackFrame::iretq`] for more info.
/// See [`__capture_set_registers`] for more info.
///
pub macro kernelcall_ret {
    ([$($reg : ident)*], $capture : expr, $frame : expr) => {
    __capture_set_registers!(($($reg),*), $capture); $frame .iretq(); }
}
pub unsafe fn example_interrupt_handler(
    _frame: InterruptStackFrame,
    _capture: RegisterCapture,
) {
    let mut response = _capture;
    response.rdx = 0x1337;
    let __macro_capture = response;
    asm!(
        "#APPLY_REGISTERS_FROM_CAPTURE\nin(\"rdx\") __macro_capture.rdx", options(nomem,
        preserves_flags, nostack)
    );
    _frame.iretq();
}
#[no_mangle]
pub extern "x86-interrupt" fn __irq_handler(_frame: InterruptStackFrame) {
    let mut capture = unsafe {
        {
            let mut rax = 0;
            let mut rbx = 0;
            let mut rcx = 0;
            let mut rdx = 0;
            let mut rsi = 0;
            let mut rdi = 0;
            let mut rbp = 0;
            let mut rsp = 0;
            let mut r8 = 0;
            let mut r9 = 0;
            let mut r10 = 0;
            let mut r11 = 0;
            let mut r12 = 0;
            let mut r13 = 0;
            let mut r14 = 0;
            let mut r15 = 0;
            asm!(
                "#CAPTURE_REGISTERS", out("rax") rax, out("rcx") rcx, out("rdx") rdx,
                out("rsi") rsi, out("rdi") rdi, out("r8") r8, out("r9") r9, out("r10")
                r10, out("r11") r11, out("r12") r12, out("r13") r13, out("r14") r14,
                out("r15") r15, options(nomem, preserves_flags, nostack)
            );
            RegisterCapture {
                rax,
                rbx,
                rcx,
                rdx,
                rsi,
                rdi,
                rbp,
                rsp,
                r8,
                r9,
                r10,
                r11,
                r12,
                r13,
                r14,
                r15,
            }
        }
    };
    let handler_index = (capture.rax & 0xFF) as u8;
    if let Some(handler) = unsafe { INTERRUPT_HANDLERS[handler_index as usize] } {
        unsafe { handler(_frame, capture) };
        {
            ::core::panicking::panic_fmt(
                format_args!(
                    "internal error: entered unreachable code: {0}",
                    format_args!("Interrupt Handler returned a value!"),
                ),
            );
        }
    } else {
        unsafe {
            {
                ::core::panicking::panic_fmt(
                    format_args!(
                        "Interrupt Handler for Index {0} is not defined!",
                        handler_index,
                    ),
                );
            };
        }
    }
}
static mut has_panicked: bool = false;
/// The Interrupt Handler for all Exceptions.
/// This function is called when an Exception occurs.
///
/// # Arguments
///
/// * `stack_frame` - The Interrupt Stack Frame.
/// * `exception_number` - The Exception Number.
/// * `error_code` - The Error Code.
///
/// # Usage
/// This function is called by the **CPU**. It's not meant to be called manually.
#[no_mangle]
#[inline(never)]
pub fn __generic_error_irq_handler(
    stack_frame: InterruptStackFrame,
    exception_number: u8,
    error_code: Option<u64>,
) {
    let exception = unsafe { exception_number.transmute() };
    match exception {
        _ => {
            unsafe {
                if !has_panicked {
                    has_panicked = true;
                    {
                        ::core::panicking::panic_fmt(
                            format_args!(
                                "Exception: {0:?}\nError Code: {1:?}\nStack Frame: {2:#?}",
                                exception,
                                error_code,
                                stack_frame,
                            ),
                        );
                    };
                }
            }
        }
    }
}
#[no_mangle]
unsafe extern "C" fn _start<'kernel>() -> ! {
    let PRIMARY_FONT: Option<limine::File> = None;
    let mut kernel_renderer = renderer::Renderer::new(
        FRAMEBUFFERS.get(0).expect("No System Framebuffers."),
        &FONT_BITMAP,
    );
    let color = graphics::Color::from_rgb(0, 255, 0);
    kernel_renderer.update_colors(Some(0xFFFFFFFF), Some(color.inner()));
    kernel_renderer.clear(color.inner());
    kernel_renderer.optional_font_scaling = Some(2);
    Renderer::make_global(kernel_renderer);
    unsafe {
        crate::KERNEL_CONSOLE
            .write_str(
                "(c) 2023 Joscha Egloff & AntOS Project. See README.MD for more info.\n",
            );
        crate::KERNEL_CONSOLE.write_str("AntOS Kernel ( ");
        crate::KERNEL_CONSOLE
            .write_str({
                b"0000000000000000000000000000000000000000 aafa4439414e5db72e63bbdc5b4a4d53b1341cfa Joscha Egloff <joscha.egloff@pm.me> 1702308920 +0100\tclone: from https://github.com/ant-os/rust-kernel.git\naafa4439414e5db72e63bbdc5b4a4d53b1341cfa 8ba1d638cf70d88819e4ab9fe965769a51660763 Joscha Egloff <joscha.egloff@pm.me> 1703009034 +0100\tcommit: Merged Local version into Remote.\n8ba1d638cf70d88819e4ab9fe965769a51660763 1b04b2ec00a1c870c59e9fc3e94d86e9f554f941 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703010265 +0100\tpull -f: Fast-forward\n1b04b2ec00a1c870c59e9fc3e94d86e9f554f941 71876d56d968156d466844257faa5895d0dce148 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703011395 +0100\tcommit: Finalized Merge\n71876d56d968156d466844257faa5895d0dce148 71876d56d968156d466844257faa5895d0dce148 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703011502 +0100\tcheckout: moving from trunk to local\n71876d56d968156d466844257faa5895d0dce148 71876d56d968156d466844257faa5895d0dce148 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703011611 +0100\tcheckout: moving from local to master\n71876d56d968156d466844257faa5895d0dce148 71876d56d968156d466844257faa5895d0dce148 Joscha Egloff <joscha.egloff@pm.me> 1703011655 +0100\tcheckout: moving from master to trunk\n71876d56d968156d466844257faa5895d0dce148 52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 Joscha Egloff <joscha.egloff@pm.me> 1703011669 +0100\tpull: Merge made by the \'ort\' strategy.\n52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 Joscha Egloff <joscha.egloff@pm.me> 1703011671 +0100\tcheckout: moving from trunk to trunk\n52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 Joscha Egloff <joscha.egloff@pm.me> 1703011744 +0100\tcheckout: moving from trunk to trunk\n52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703011775 +0100\tcheckout: moving from trunk to trunk\n52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703011816 +0100\tcheckout: moving from trunk to trunk\n52b9b6143e87e8d17ef514640d2a3e00dba2d8e1 2528423bac32c819ae275266385ed0985d29d74f Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703013350 +0100\tpull --tags origin trunk: Fast-forward\n2528423bac32c819ae275266385ed0985d29d74f 2528423bac32c819ae275266385ed0985d29d74f Joscha Egloff <joscha@DESKTOP-UREG7P1> 1703020367 +0100\tcheckout: moving from trunk to refactoring\n2528423bac32c819ae275266385ed0985d29d74f efbbb1bae8ad9bc893d9e45dd5900a4eb5df526b Joscha Egloff <joscha.egloff@pm.me> 1703020470 +0100\tcommit: Fix memory allocation and mapping issues\nefbbb1bae8ad9bc893d9e45dd5900a4eb5df526b ec9100cde5c851aabcda11fda08093c48c3b27b6 Joscha Egloff <joscha.egloff@pm.me> 1703021732 +0100\tcommit: Update README.md with AntOS Kernel Rewrite project details\nec9100cde5c851aabcda11fda08093c48c3b27b6 2528423bac32c819ae275266385ed0985d29d74f Joscha Egloff <joscha.egloff@pm.me> 1703071397 +0100\tcheckout: moving from refactoring to trunk\n2528423bac32c819ae275266385ed0985d29d74f 0f315e6d10c8ddb07235cf7bb106bed3c8fad088 Joscha Egloff <joscha.egloff@pm.me> 1703071419 +0100\tpull --tags origin trunk: Fast-forward\n0f315e6d10c8ddb07235cf7bb106bed3c8fad088 0f315e6d10c8ddb07235cf7bb106bed3c8fad088 Joscha Egloff <joscha.egloff@pm.me> 1703071450 +0100\tcheckout: moving from trunk to gdb-protocol\n";
                b"DIRC\x00\x00\x00\x02\x00\x00\x00\\e\x81\xdf\xd9*%J\x98e\x81\xdf\xd9*%J\x98\x00\x00\x00M\x00\x02\x96\xb5\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x06\xcf\xad\xcdQ\x1b\xc4\xaf`\x80\xfe\xca\x89\xb5eb\xa1\xe7\xc2\x902\xab\x00!.github/workflows/rust-clippy.yml\x00e\x82\xce\xb99T\x10\xa8e\x82\xce\xb99T\x10\xa8\x00\x00\x00M\x00\x00\x0f\xb4\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\x95\xe4*]\t\x1c\x13\xa0\xd8\x82\xa9\x84\t\x8f)r\x85\x17\xf2\x02\x8c\x00\x1a.github/workflows/rust.yml\x00\x00\x00\x00\x00\x00\x00\x00e\x80}\x0e\x07\xec\xac\xa0e\x80}\x0e\x07\xec\xac\xa0\x00\x00\x00M\x00\x05\xbf\r\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x9e<\xa0\xdd#\xee\x9b\x95!\xf4\x82\x82\xc0\xf8\xf14k\x1d\xfcV\x88\x00\x15.vscode/settings.json\x00\x00\x00\x00\x00e\x82\xce\xb9:E\xe74e\x82\xce\xb9:E\xe74\x00\x00\x00M\x00\x02%\xa2\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x14e7B\x9c\x8a\x1e\xb8\xe3\"\x0e\x99\xa3\x99(v\xa0\x9d1\x9cr\x81\x00\x12CODE_OF_CONDUCT.md\x00\x00\x00\x00\x00\x00\x00\x00e\x82\xce\xb9; \x03\xece\x82\xce\xb9; \x03\xec\x00\x00\x00M\x00\x028\xb1\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x06<\x07\x8b6\x9f\x1a\xb7\x92\x84\'\xcb4\xe5\xf0e7\x8a\\\xc9\xc8\xbc\x00\x0fCONTRIBUTING.md\x00\x00\x00e}\xaaS%I\xf8\\e}\xaaS%I\xf8\\\x00\x00\x00M\x00\tG@\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\r\xa7\xd7.\xdep\'\xd8\xa1%\x96\xa56\xfawJC\xed\x00:\x83\xd3\x00\x0bGNUmakefile\x00\x00\x00\x00\x00\x00\x00exq\xc0-\x9c#\xccexq\xc0-\x9c#\xcc\x00\x00\x00M\x00\x01e\xe9\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x0c\xc5~\xffU\xeb\xc0\xc5Is\x90:\xf5\xf7+\xacrv,\xf4\xf4\x00\x08INIT.SYS\x00\x00ew,8\'\xe0\x02dew,8\'\xe0\x02d\x00\x00\x00M\x00\tGO\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xaa\x8a5\xb8\xf86p\xacX\xe50Go\x8b\xcdI\x01\xb6\xc2J\x1c\x00\x07LICENSE\x00\x00\x00ey\xcd\xdc6\x12\xfe\xa4ey\xcdy\x19e\xc2\xc8\x00\x00\x00M\x00\x02W\x1a\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x01 \x00B\xd1l[\xd5\x07?m\xd1\x13\xb1V\xc8&\x1b\xda\xa4cM\xd0\x00\x08LOGO.SYS\x00\x00e}\xaa-\x06D\xe5\xe8e}\xaa-\x06D\xe5\xe8\x00\x00\x00M\x00\x00D\x83\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x1f\xd0\xa0\x88\xa7\xf1\xf5\xe2\xa3u~p\x9a\xaa)i\xab\x9b\x9f\xbc\x0cN\x00\x0bPROGRAM.SYS\x00\x00\x00\x00\x00\x00\x00e\x82\xce\xba\x01H\x07de\x82\xce\xba\x01H\x07d\x00\x00\x00M\x00\x00\xafn\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x05LC\xc7[\x92F\xd8\x90\xec\x14\x7f\xa4\xc0\x16iSo\xf2\x8d\xdc2\x00\tREADME.md\x00ew,8\'\xf1,\xecew,8\'\xf1,\xec\x00\x00\x00M\x00\tGQ\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00\x05a.out\x00\x00\x00\x00\x00e\x7fV\xca&\x1bZPe\x7fV\xca&\x1bZP\x00\x00\x00M\x00\x00\x1b\x9c\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x04\x00\x00\x00D7\xedC\xef\xc1\x85Z9v\xe8\xb2\xeeQx\xb0lB2\x1a\x00\tantos.hdd\x00e\x82\xce\xba!\x03m<e\x82\xce\xba!\x03m<\x00\x00\x00M\x00\x00\xb3*\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x92\xe0\x00}@\x13}\nm\x1cPt\xc2\x97\x10\x04\xfa\xad\xcb\x00\xa2wT\x00\tantos.iso\x00ex\xb6\x17\x08W\xb9\xb4ex\xb5\xf0491H\x00\x00\x00M\x00\x00\xac\xcc\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x01\xe5\xa9\x92\x08\xeb\xd0\x18\xaa\x07\xea\x01\xad]F\xd0\x14\x1f\x911\xb5\xec\xa6\x00\x1bkernel/..kernel_expanded.rs\x00\x00\x00\x00\x00\x00\x00ew,8(\x00sxew,8(\x00sx\x00\x00\x00M\x00\tGS\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x10\x00EW\xed\xe2%\xe0\xdfh\x94\x90\x08*\xe2\x01\xa5\x1d\xa2 \xf1\x00\x11kernel/.gitignore\x00e\x82\xce\xba#&\x00He\x82\xce\xba#&\x00H\x00\x00\x00M\x00\x00O\xb9\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00 q\xe2\xb5\x7f\x1f\x88T?\xeb\x93\xb3\xc2\\6n\xeau\xbb\x80\xb6\x86\x00\x11kernel/Cargo.lock\x00e\x80p\x06\x0e\xd0\x13\xe0e\x80p\x06\x0e\xd0\x13\xe0\x00\x00\x00M\x00\x01\xcd\x8a\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02e\xe2i\xc8.q\xeb\xb6Y\xcb\"\xf1\xc3\xcd\x17x~UNj5\x00\x11kernel/Cargo.toml\x00exo\xb9\x14\x90\xa4pexo\xb9\x14\x90\xa4p\x00\x00\x00M\x00\tGV\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02e8\xa2\x99\xb8\xe1\xd4\xbf\n\x88\xae\x9f\xc0\x13\x11\x83\x8e\xe5\xec\xd5\x10\x00\x12kernel/GNUmakefile\x00\x00\x00\x00\x00\x00\x00\x00ew,8(\x1e\xfc\xa8ew,8(\x1e\xfc\xa8\x00\x00\x00M\x00\tGW\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xda\x92\xb8[\x12\xb5\xd7\x0c\x92=]6\xc9\xa1`Q\xab\xf0\x84t\xfa\x00\x0fkernel/build.rs\x00\x00\x00ew,8(.4\xc0ew,8(.4\xc0\x00\x00\x00M\x00\tGX\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x07\x8cj\x8b\xe2\x9dQ\xd4T\xb6\x00/\n\xa7\x12\n\xaar\xf7\xafY?\x00\x10kernel/linker.ld\x00\x00ew,8(=u\x0cew,8(=u\x0c\x00\x00\x00M\x00\tGY\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00E\x91\x83\xcb\xf3\xd2U\x16\xf3\xbf,\xa6\xf3y\x7f\xbfxL\xa9\x0e\xef\x00\x1akernel/rust-toolchain.toml\x00\x00\x00\x00\x00\x00\x00\x00e\x82\xce\xba%\x0e\xb2\x88e\x82\xce\xba%\x0e\xb2\x88\x00\x00\x00M\x00\x01\x0e\x9e\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x11\x85\xa5\r\xe3Q\xd2\xd5\r\xd1P\xa3\xdd\xa93c\xaf\xe7ql\xa3l\x00\x18kernel/src/alloc_impl.rs\x00\x00ew-\'\x07N\xa1@ew,8(N\x17@\x00\x00\x00M\x00\tG[\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00#\x86\x9bx\xde\xc3DH\x90\xa8\xb6^\xc4L0\x82\x14`\xdb\x91\x86\x80\x00\x15kernel/src/bitmap.raw\x00\x00\x00\x00\x00ey\xae\xa540\xcc\x10ey\xae\xa540\xcc\x10\x00\x00\x00M\x00\tG\\\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\xa1}\xe4Dj\xb1\xa7\xd4\xc0P\n\x9eJ\xd6#\xf6.u\x86\x11\x8e\x00\x19kernel/src/bitmap_font.rs\x00ew,8(]b\x18ew,8(]b\x18\x00\x00\x00M\x00\tG^\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00J\x84\xd8k\xc5z\x16\xb0\x9d&p\xdezl\x0fp(\r\xf1\xd4\xc7\x00\x1bkernel/src/common/consts.rs\x00\x00\x00\x00\x00\x00\x00e~\xeeC\x1e\x1a\x19\xb8e~\xeeC\x1e\x1a\x19\xb8\x00\x00\x00M\x00\x00^[\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x07_U\xce\xae\"\xd4R\xc7\x04\xb0\xb1F6G\xfa\x88\xcbm\x17d\x84\x00\x18kernel/src/common/gdt.rs\x00\x00e|\xb0\xd3\x13\xfd\xe9(e|\xb0\xd3\x13\xfd\xe9(\x00\x00\x00M\x00\nK\xb9\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\xc0\xd4\"\'\\B\x90w\xc1\x0cc:[\x8eJ\x82\x83W\x1b\x82\x91\x00\x1ckernel/src/common/handler.rs\x00\x00\x00\x00\x00\x00e\x82\xce\xba&\xec\xf94e\x82\xce\xba&\xec\xf94\x00\x00\x00M\x00\x01Ck\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\nh\xda\xe2j\x03\xec\xb1\nw\xc9\xa4\xe1\xedVs\xa3*\x89\x86\x11j\x00\x18kernel/src/common/idt.rs\x00\x00e}\x91\xd8\x02\x85P\xc8e}\x91\xd8\x02\x85P\xc8\x00\x00\x00M\x00\tG_\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\xb66\xb2\xd8\xd6I\xbfrLQg\x04\x17+\xcd\x01;V[\x89k\x00\x17kernel/src/common/io.rs\x00\x00\x00ew,8({\xee\xccew,8({\xee\xcc\x00\x00\x00M\x00\tG`\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x02\xd3\xf5\xa1/\xaa\x99u\x81\x92\xec\xc4\xed?\xc2,\x92I#.\x86\x00\x1bkernel/src/common/macros.rs\x00\x00\x00\x00\x00\x00\x00e\x82\xce\xba(T\xfb\xb4e\x82\xce\xba(T\xfb\xb4\x00\x00\x00M\x00\x01_\x9d\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x188\x9b3\xcan\xa5]\x0f\x8f\"\x8eq\x1eo=/0\xa7D\xbb\x8d\x00\x18kernel/src/common/mod.rs\x00\x00ew,8(\x8c\xe0Lew,8(\x8c\xe0L\x00\x00\x00M\x00\tGc\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00 \xa8\xac\xdf\x8cJ2\xa8\xa1\xdc\x9b,\x89eV:\xb3\xa1\xd24\xeb\x00\x1akernel/src/device/block.rs\x00\x00\x00\x00\x00\x00\x00\x00e}\xdd\xd8\x08\xad\x1b\x84e}\xdd\xd8\x08\xad\x1b\x84\x00\x00\x00M\x00\tGd\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02\xae\x14\xf4\x17\x16\xe5\x0ep\xbb\xb5\xb4\xd1/8f\xefY\x0e`F\xd2\x00\x1ekernel/src/device/character.rs\x00\x00\x00\x00ew,8(\xaba\xacew,8(\xaba\xac\x00\x00\x00M\x00\tGe\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00 \xe2\xcb\x12\x0e\x82JI\xa8\x9b\xbc\xe9\xbd\x15\x92\xe7\x86\xb1\xd0\x85\x83\x00\x1akernel/src/device/input.rs\x00\x00\x00\x00\x00\x00\x00\x00ew,8(\xaba\xacew,8(\xaba\xac\x00\x00\x00M\x00\tGf\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\xa8(\xc2NF\x15\xd72i1e\x88\xfbjC6\r\xdc\x92$\xbc\x00\x18kernel/src/device/mod.rs\x00\x00ew,8(\xba\xa8\x9cew,8(\xba\xa8\x9c\x00\x00\x00M\x00\tGg\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\"n\xe0d\xcf\x835\xe6\x12\xe3[\x0f`bA\xca9S\xaf,3\x00\x1ckernel/src/device/network.rs\x00\x00\x00\x00\x00\x00ew,8(\xc9\xe8\xe8ew,8(\xc9\xe8\xe8\x00\x00\x00M\x00\tGh\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x1e{^:\x9a\x00\x83\xa4\x06l\x0b\xb6\\\x1a,X2<\xdc\xdb\xb9\x00\x19kernel/src/framebuffer.rs\x00e\x80p-\x12\x9c\xd78e\x80p-\x12\x9c\xd78\x00\x00\x00M\x00\x00*\x94\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xe9x\xf0z\x972\x04\x88-\xe8 l9\x92\xad\x08\x8a\xfb\xa3MQ\x00\x1dkernel/src/graphics/buffer.rs\x00\x00\x00\x00\x00e\x80q\xca\x08\xdc\xa5\x0ce\x80q\xca\x08\xdc\xa5\x0c\x00\x00\x00M\x00\x00\xacB\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02\xa4\x12\x94\x9b1\xa9Ay\x86\xac`\xdd\x0c\\\xc7\xf2\xdb\xcb\xa9o5\x00%kernel/src/graphics/buffer_manager.rs\x00\x00\x00\x00\x00e\x80|\xb3\x08b\x83le\x80|\xb3\x08b\x83l\x00\x00\x00M\x00\x02MV\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x05H\x1f\x84\xd0\x19]\xee~s\xc88\xc4\x88\x8d7\x10/\x8do7Z\x00\x1akernel/src/graphics/mod.rs\x00\x00\x00\x00\x00\x00\x00\x00e|\x99s\x00E-`e|\x99s\x00E-`\x00\x00\x00M\x00\nKQ\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02\xf9\xf9r\xa0|\xd3w\x13>O\x066M\x8f\xdcE\x0f\x92w1\x82\x00\x1bkernel/src/kernel_logger.rs\x00\x00\x00\x00\x00\x00\x00e}\x8d\xc2)\x91sTe}\x8d\xc2)\x91sT\x00\x00\x00M\x00\x00\xa9x\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x07\xd6\xffj\r\xd7\xe9?/\x03\xab\t\xf3\xb2\x83#\xa9\xd7VUM3\x00\x18kernel/src/legacy_pic.rs\x00\x00e\x82\xce\xba*\x97L\xece\x82\xce\xba*\x97L\xec\x00\x00\x00M\x00\x01_\xb3\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00>\xf7K\x16\xa7\xf7\xfe\xb1\x93\xb4\x0e%0%Z\xe5\xa9L\x902|9\x00\x12kernel/src/main.rs\x00\x00\x00\x00\x00\x00\x00\x00e\x80\x1fi1\xa8\xf7$e\x80\x1fi1\xa8\xf7$\x00\x00\x00M\x00\tGj\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x0b\x9f;\xcc_aA\xb5\xb9\xb4:g;x\x99\x87Rv)\tq\x9d\x00\x14kernel/src/memory.rs\x00\x00\x00\x00\x00\x00ew,8(\xdb#tew,8(\xdb#t\x00\x00\x00M\x00\tGk\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00\x11kernel/src/mod.rs\x00e\x82\xce\xba-<Y\xece\x82\xce\xba-<Y\xec\x00\x00\x00M\x00\x01`\xee\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x005\xbf\"\xc1\x18\xfeA6Yu\x19\xc1\xec5\x19kl-\xc4R\x03W\x00$kernel/src/paging/frame_allocator.rs\x00\x00\x00\x00\x00\x00ex\xa1\xe0\x04sf\xb8ex\xa1\xe0\x04sf\xb8\x00\x00\x00M\x00\x02A\x10\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x02H\xef\xedo\xb5P\xdd\xd9\xc8p\x87\xa6\xf1\x8c\xda\xc3\xd5l\xd8\xc9\xd8\x00\x1ckernel/src/paging/indexer.rs\x00\x00\x00\x00\x00\x00e\x82\xce\xba1\xcfJ\x14e\x82\xce\xba1\xcfJ\x14\x00\x00\x00M\x00\x01g\xe7\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\t\x9b\x7fj\xf7\x87\xb6\xdd\xc3\xf4\x9d+:\x1c\x15\xe8\xbd\xf7\xed!\xd0\xd4\x00\x18kernel/src/paging/mod.rs\x00\x00e\x82\xce\xba5\x8c\xc1he\x82\xce\xba5\x8c\xc1h\x00\x00\x00M\x00\x00\x94a\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x0b\xef \xb3\xe7\xe9\xc2\xa3\xa9m\xf2f\xb9\xc6\xdaa\x1c\xa5;y\xe9j\x00\"kernel/src/paging/table_manager.rs\x00\x00\x00\x00\x00\x00\x00\x00e}\xc3J\x04\xe6\x81\xe8e}\xc3J\x04\xe6\x81\xe8\x00\x00\x00M\x00\tGo\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x1f\x04u\x8eR\xab{\xf4A\x05\xe4\x1d\xea\xc9T\x04\x9a\xe8\x9fw\xf7\xd9\x00\x16kernel/src/renderer.rs\x00\x00\x00\x00e}\xcbd$Gu\x10e}\xcbd$Gu\x10\x00\x00\x00M\x00\tGp\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x0e\xc3\x84P\xa7\xcb\xec88B\xc5\x0b\xc2\xa9y\xfe\xc8o\x88AF\xf7\x00\x14kernel/src/serial.rs\x00\x00\x00\x00\x00\x00ew,8)\x184\xa8ew,8)\x184\xa8\x00\x00\x00M\x00\tGq\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x02\xd3\xf5\xa1/\xaa\x99u\x81\x92\xec\xc4\xed?\xc2,\x92I#.\x86\x00\x14kernel/src/status.rs\x00\x00\x00\x00\x00\x00e}\xc3\xce\x1aM\xc8\xd4e}\xc3\xce\x1aM\xc8\xd4\x00\x00\x00M\x00\tGr\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\n[\xefQ\xcf+\x0c\xf43[\xd4\x96\x9c$\xdf6\xfbm2\x80\x04\n\x00\x11kernel/src/tty.rs\x00ew-z\x16\x0e\x81\xdcew-z\x16\x0e\x81\xdc\x00\x00\x00M\x00\x02&#\x00\x00\xe0\x00\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x10\x00\x98l\x9a\x99jg\xcdM\xf2\x14\xddfA\xb0\xdd\xb3\xde\xac0\x99\x00\x06limine\x00\x00\x00\x00e}\xaa}\x1d\xcc.\x10e}\xaa}\x1d\xcc.\x10\x00\x00\x00M\x00\tGs\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x04l\xbe\xdd\xd7\xd9\xdb\x11\x17\x9f\xbd\x953\x86d\xae\xf8\x0f\xce\x08:\x92\x00\nlimine.cfg\x00\x00\x00\x00\x00\x00\x00\x00ey\xf1\x1a\x17\x80Odey\xcab6P\x1b\xf4\x00\x00\x00M\x00\x02I\x15\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x06\x8e[E\xb3\xa7\xaf\x81\xbepb\xdb+\xf3\xe38\xfa\x01\'(\x84\xc8\x00\x08logo.jpg\x00\x00ew,8)8\xa8\xdcew,8)8\xa8\xdc\x00\x00\x00M\x00\tGt\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xd7;\x86\xb3\x18\xed\xd0\x1a\x02YO!bx\x15\x1f\x0f(\x87\xf4\xfa\x00\x07out.txt\x00\x00\x00e{l\x9c$\xbe\xe6\xcce{l\x9c$\xbe\xe6\xcc\x00\x00\x00M\x00\x02F\x1f\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00@\x00\x00\xa2[\x89\xcc\xccB\xe2\xbd\x90\xff\xdd\xc9\xc0\x14V).B\xd3\xe2\x00\x0covmf/OVMF.fd\x00\x00\x00\x00\x00\x00e}\xa5\xc97M>\xb0e}\xa5\xc97M>\xb0\x00\x00\x00M\x00\x00R*\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00\x14programs/GnuMakefile\x00\x00\x00\x00\x00\x00e}\xa5\xe3)\xe9\xd7te}\xa5\xe3)\xe9\xd7t\x00\x00\x00M\x00\x007?\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00\x11programs/build.py\x00e}\xa7;\x18&-\xe4e}\xa7;\x18&-\xe4\x00\x00\x00M\x00\x00R\xa2\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x9f;\xcb\x98Z\x9eH\x16\xfa\xb0\xbd\xac\xb8-\t=\xff\x89\xb4Ob\x00\x1bprograms/example/Cargo.lock\x00\x00\x00\x00\x00\x00\x00e}\xa6\x1a\x177\xb8\xf8e}\xa6\x1a\x177\xb8\xf8\x00\x00\x00M\x00\x00R<\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xb8\xaa\xe9\x89\x964\xe5D]\xfc\xb5\x85}\t\xabK\xd0\x1dd\xda\xde\x00\x1bprograms/example/Cargo.toml\x00\x00\x00\x00\x00\x00\x00e\x7fZ: go@e\x7fZ: go@\x00\x00\x00M\x00\x00RD\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x04\t\xe8_<E\x03oQm\x13sK\xbd\xd2q\xb4\xfeh\xe8\xe3\x15\x00\x1cprograms/example/src/main.rs\x00\x00\x00\x00\x00\x00e}\xa7[0\x9e\xcc\xdce}\xa7[0\x9e\xcc\xdc\x00\x00\x00M\x00\x00]\xaa\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x0c\\\x8c\xfeF\xfcI\x05zOF\x8a\xe9b\xdfM<)\xe3\x0f\xcc\xc7\x00(programs/example/target/.rustc_info.json\x00\x00e}\xa7;\x19\x982\x1ce}\xa7;\x19\x982\x1c\x00\x00\x00M\x00\x00R\xc7\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xb1 \xd7\xc3\x19\xcd\xa9E\xdc\x07r\x97\x97u\nI\xb22 j\xb5\x00$programs/example/target/CACHEDIR.TAG\x00\x00\x00\x00\x00\x00e}\xa7;\x19\xf3\xbc\xe0e}\xa7;\x19\xf3\xbc\xe0\x00\x00\x00M\x00\x00R\xe5\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00)programs/example/target/debug/.cargo-lock\x00e}\xa7;\x1aD\x7f\xe0e}\xa7;\x1aD\x7f\xe0\x00\x00\x00M\x00\x00S\x03\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xb1 \xd7\xc3\x19\xcd\xa9E\xdc\x07r\x97\x97u\nI\xb22 j\xb5\x008programs/example/target/x86_64-unknown-none/CACHEDIR.TAG\x00\x00e}\xa7;\x1a\xafR\xc0e}\xa7;\x1a\xafR\xc0\x00\x00\x00M\x00\x00S\x11\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00=programs/example/target/x86_64-unknown-none/debug/.cargo-lock\x00\x00\x00\x00\x00e}\xa7;\x1b\xf4\xb50e}\xa7;\x1b\xf4\xb50\x00\x00\x00M\x00\x00S\xe5\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x000\xe0\x03(\xdaZ\xa8\xe7\xfb\xa80\xf8\xcc\x8d\x04wvF\xc3l\xff\x00qprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-90cda0d796f5b5ce/invoked.timestamp\x00e}\xa7;&\x95\xf4@e}\xa7;&\x95\xf4@\x00\x00\x00M\x00\x00]\x91\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x14\xc6\xfc\xd9;^\xb0\xfe\xd9\x04\xa2\x1d\x8c\xac\xdf\xfa:{\xe4W\xeao\x00zprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-90cda0d796f5b5ce/output-bin-example-program\x00\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfd\x01k\xe8\xf0e}\xa9\xfd\x01k\xe8\xf0\x00\x00\x00M\x00\x00c\xa8\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x10\xfd\x8aWE\xb6\xaa\x80^4\xd0x\x97\xc1?TW7k4\xb2\x00sprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-b883f15776df51a0/bin-example-program\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfd\x01\xa5\x81xe}\xa9\xfd\x01\xa5\x81x\x00\x00\x00M\x00\x00kV\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\x9f\xfa.\xe9\x949[d\xf7\x9c|\xf8e\xce\x12b\xea\xcak\xcbI\x00xprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-b883f15776df51a0/bin-example-program.json\x00\x00e}\xa9\xfd\x00\x8b\x04He}\xa9\xfc\x00\x00\x00\x00\x00\x00\x00M\x00\x00^S\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x18_\xdf\x10?n\x82\xe8$\x11\x91!\xc5\x9a\x0c\xe8\xe7\xc2\xaf\x99%\x00wprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-b883f15776df51a0/dep-bin-example-program\x00\x00\x00e}\xa9\xfc*\x99\x03\x98e}\xa9\xfc*\x99\x03\x98\x00\x00\x00M\x00\x00^\x12\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x000\xe0\x03(\xdaZ\xa8\xe7\xfb\xa80\xf8\xcc\x8d\x04wvF\xc3l\xff\x00qprograms/example/target/x86_64-unknown-none/debug/.fingerprint/example-program-b883f15776df51a0/invoked.timestamp\x00e}\xa7;#\x7fF\xf8e}\xa7;#\x7fF\xf8\x00\x00\x00M\x00\x00]\xa7\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\t\'\xc0\xf4>\x15w\xb5\x0e\x02\xbb\x83[(|\x92]\x19\xa0\x86\xb4\x00Yprograms/example/target/x86_64-unknown-none/debug/deps/example_program-90cda0d796f5b5ce.d\x00e}\xa9\xfd\x01.\xd2\xa8e}\xa9\xfc:\x91_\x84\x00\x00\x00M\x00\x00A\x17\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x1f\xd0\xa0\x88\xa7\xf1\xf5\xe2\xa3u~p\x9a\xaa)i\xab\x9b\x9f\xbc\x0cN\x00Wprograms/example/target/x86_64-unknown-none/debug/deps/example_program-b883f15776df51a0\x00\x00\x00e}\xa9\xfc1\xc8\x90\xfce}\xa9\xfc1\xc8\x90\xfc\x00\x00\x00M\x00\x00a\x9c\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x01\t\xaf9\xa6M\x08\xbdC\x90|\xc65\xa3q\x0b\x95\x7f\xe8\x97\xd3L\x00Yprograms/example/target/x86_64-unknown-none/debug/deps/example_program-b883f15776df51a0.d\x00e}\xa9\xfd\x01.\xd2\xa8e}\xa9\xfc:\x91_\x84\x00\x00\x00M\x00\x00A\x17\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x1f\xd0\xa0\x88\xa7\xf1\xf5\xe2\xa3u~p\x9a\xaa)i\xab\x9b\x9f\xbc\x0cN\x00Aprograms/example/target/x86_64-unknown-none/debug/example-program\x00e}\xa8\xef#7\xa8\xfce}\xa8\xef#7\xa8\xfc\x00\x00\x00M\x00\x00k\x9b\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x8e?\x17o\xae\xcb`\x83?\xa1\xb7\xf5\xdf\x93g22xN\xc1\xd6\x00Cprograms/example/target/x86_64-unknown-none/debug/example-program.d\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfc:\xf7Ghe}\xa9\xfc4\x9fr\xa0\x00\x00\x00M\x00\x00R\x04\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00,\x90\xc0\xf0_\xb4j\x1a\n\x87\xf2\xb8\tKg\x1d\x1a\xf5\xb7\x7fHH\x00\x9bprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4/1pf5l98mhuxb4c40.o\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfc;\x17\x8b,e}\xa9\xfc4i\xb0\x84\x00\x00\x00M\x00\x00R\x06\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x0e \x96/\x06\xbe<d\xa5\x80\xbb\xc1\xce\x97\x13j\xeeJ\xa8Y\x94\xea\x00\x9bprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4/33fqgkfa8krixqrn.o\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfc4\x9fr\xa0e}\xa9\xfc4\x9fr\xa0\x00\x00\x00M\x00\x00Q\xf0\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\xb6\xd7\x15N1eKo\x97\xd2f7\xceXZ{VQ36)9\x00\x96programs/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4/dep-graph.bin\x00\x00\x00\x00e}\xa9\xfc4\x90\n\x18e}\xa9\xfc4\x90\n\x18\x00\x00\x00M\x00\x00R\x0b\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x19PX\x81BY\x9e\xfb\x85Z\x05.\x95\xcfYv\xbf~j6\n\x1f\x00\x98programs/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4/query-cache.bin\x00\x00e}\xa9\xfc5\xad\xb7Le}\xa9\xfc5\xad\xb7L\x00\x00\x00M\x00\x00R\x0e\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\xa1\xe3+e\x8a\xd2\xcb\x06\x8c\x01!I\x17\xfb\xc3*\xbf+\xf5\x85L\x00\x9aprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4/work-products.bin\x00\x00\x00\x00\x00\x00\x00\x00e}\xa9\xfc.y^\xf4e}\xa9\xfc.y^\xf4\x00\x00\x00M\x00\x00Q\xbe\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00sprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-9ic89uw4p6xg/s-grkhcqfsoy-y9b5z6.lock\x00\x00\x00\x00\x00\x00\x00e}\xa7;&\x86\xd1@e}\xa7;&\x86\xd1@\x00\x00\x00M\x00\x00]\x8f\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x13N\xe5\x17\xf1\x8c\xe1\xbd\xf2\x9d\xb7\xf0\x18\xb1\x89\xa4u\xfb\x0bs \xe4\x00\x8aprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-jn9stlakvifm/s-grkh12jva9-17nr2z0-working/dep-graph.part.bin\x00\x00\x00\x00\x00\x00\x00\x00e}\xa7;\x1f\xa2\xcb\xcce}\xa7;\x1f\xa2\xcb\xcc\x00\x00\x00M\x00\x00TW\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x00\x00\x00\xe6\x9d\xe2\x9b\xb2\xd1\xd6CK\x8b)\xaewZ\xd8\xc2\xe4\x8cS\x91\x00tprograms/example/target/x86_64-unknown-none/debug/incremental/example_program-jn9stlakvifm/s-grkh12jva9-17nr2z0.lock\x00\x00\x00\x00\x00\x00ey\xf1\xd5\'\xd9?$ey\xf1Q\x0c\xae\xa8\xec\x00\x00\x00M\x00\x01\xd3\xbd\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x07\x01\xac\xe8\x81\xcaKY\xfe\xe7\x0c\x0b<\x87\xb2X\xa5\xab\t\x06^\xb5\xe5\x00\x1bresources/fonts/CONSOLA.TTF\x00\x00\x00\x00\x00\x00\x00ey\xf1\xd5,q\t|ey\xf1Q\x17a\x1dt\x00\x00\x00M\x00\x07Y\x9d\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x06\x12Hw\xf5\xd6\x05.\x1b\x02\xa1G@?s\xf3\x84\xc9\xa9^d\xf2\xe8\x00\x1cresources/fonts/CONSOLAB.TTF\x00\x00\x00\x00\x00\x00ey\xf1\xd5/NgHey\xf1R\x01em\xf4\x00\x00\x00M\x00\tA\x0f\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x07\"\x80-\xe4\xde\x8a\x99\x0f(\x93C\xc9\xe0P\xa9L\xd6\xd3#\xaf}v\x00\x1cresources/fonts/CONSOLAI.TTF\x00\x00\x00\x00\x00\x00ey\xf1\xd52e\x97\x08ey\xf1Q+W\xfa\xb4\x00\x00\x00M\x00\t9\xf9\x00\x00\x81\xa4\x00\x00\x03\xe8\x00\x00\x03\xe8\x00\x069\x94\xd9\xdf!\x10\xfe\xe1\xc4\x88\x12\x96\xa2\xd9\x15\x0c\x11a\xd7\xb8H\xcb\x00\x1cresources/fonts/CONSOLAZ.TTF\x00\x00\x00\x00\x00\x00TREE\x00\x00\x04:\x0092 6\n\xe0\xd9,}\x12\x0b\xb4\x0b\x911v\x97)\x00\xf1\x9d|\xef\xd1rovmf\x001 0\n\xdc\xd0\x90\xbf\xb1\xda5@;\xad\xbf\xbb\xa7X\xcdN*\xc8\xaf!kernel\x0040 1\n\xd0?\xda\x165\xa2:6\x00\x1c\"F\xa0\x05T\xa5\xb4) gsrc\x0032 4\n~P=\xf0\x9f2j&\xfb\xc0%\x89\x81\xebk>\xf2\x8e\xf4:common\x007 0\n\x05V\xf2D\xa5\xbd\x8bICe\xe7\xea\x1aJGi#\x95\xa3\xc0device\x005 0\n|*0\xf0\x96v\xad\x13\xfbh\xc4\x98\xa5\xfa.\xf5^\xef\xfc\x9bpaging\x004 0\n\xd2\xfc\x90)nb\xac\x07P\xd2_h\x93x`P\xe8\x99?\xfdgraphics\x003 0\n\x8d\xed\xeb{-\xab6O9\xccGW\x9e\xc0\xbdN\xfe\xcf\x16r.github\x002 1\n\xe2=\x99\xab\xbf\x1e\xc1\xe2\x92=\x8f\xb4Q\xc7\xd3\x18\xe5HyGworkflows\x002 0\n~.x\xcb\x00`7\x15\xd5\x1e\xf3Q\xd4]`\xae1n\x99c.vscode\x001 0\n/\x06\x82\xf0\x0fH\x1f\xb0\x16G\xb6\x87\xd7\xb2\x85\xd1\x14\xfe\xf4\xe4programs\x0029 1\n\x88\xde\x81\xae\xdc\x7f\x17q\x97\x8f\x9a\x1eU\xce\x1c\x89o\xda\xa3\x8dexample\x0027 2\n\x15\x9d\xdc\xffOfM43Q\xa4\x9a)\x03%\nDZ\x04\x1asrc\x001 0\n\x1c\x97\xa1\x0c\xbe#\x10%0\xed\xd4\t\xe0\x8d\xe1N\xe4\xf5\x89\xd9target\x0024 2\nJ.J\xfa\xa7\xa4\xa9a1\x18O\xf5\x166\xde\xa6;\\\xc2Sdebug\x001 0\n_\xc8\x1b\x9e\x90\x80$R\xe2v\xe2\x958B$%\xcc\x9a\xa0\x92x86_64-unknown-none\x0021 1\n\xb8\xf6\x08#M\xdd\xe3\xae\x8b3\x18\xc4\x13\xcf{.\xb9a\"\xc0debug\x0020 3\n6\x08gL\xf6\xad\xdb\x07\r8\xe1\xabD\xc4\xdd\x05\xba]V\xa9deps\x003 0\n\xf1\x07\xcee\xdd\x85j\x17\x1b\xb9l\x0b\xbe\x94*i4\xc8q\xb9incremental\x008 2\n\x93\x94\x1a\x1fw\xcd$\xb4_\\\xf0\x9ee\x9e\x85\x00\'i\xc8\xb5example_program-9ic89uw4p6xg\x006 1\n\x0c\t\xc5\xf8\xd3\xe9\x94\xa3!i\xddN\x9e\xcb\x00\xf15\xf5A\xb6s-grkhcqfsoy-y9b5z6-c6mbj55ukyip8nfzu35p39ux4\x005 0\n\x8e\'\xaf\xba\x11C4\xdf\x18bk\x96\xa2\x86\xae\xb8k\x14\x0c5example_program-jn9stlakvifm\x002 1\n\xcb\xc3\xcd\x86\n\xbd\x02ie\xc5g\x07\xd1L)x\xdc\tA\xf0s-grkh12jva9-17nr2z0-working\x001 0\nf\xe8R\x98u,<\xcc5#w/\x89\x1d\x13\xd9\xaf\xb4\xb7Z.fingerprint\x006 2\n!\x81X\xdd\x89\xe5N\x1e\x97\x91\xa4\x8e\xcb`I\xf0\xe0\xdag\xaeexample-program-90cda0d796f5b5ce\x002 0\n\xc3a\xde6\xdd|\'<\xcd\x00\xf0\xae\xd3\xa5\xb5\x045\xb1\x87Nexample-program-b883f15776df51a0\x004 0\nY%\xc3Sk\r-\x1cU)4\xfc\xff\\\xf9\x1f<l\x06}resources\x004 1\n.\x08wW\\`4\xd7\x90K\xd1w\xc8&?~A\x06\xceWfonts\x004 0\nq\xee\xf7=\x154\x9c\x9a\x190\xe8\xdb\x19\x1a\xdd\x19!q\xe7*\xe0f\"id\n>\xd4!Scy\xf6x\xc7\xd6\x04\xf0I-";
                "0f315e6-modified"
            });
        crate::KERNEL_CONSOLE.write_str(" )");
        crate::KERNEL_CONSOLE.newline();
    };
    KERNEL_CONSOLE.newline();
    unsafe {
        crate::KERNEL_CONSOLE.write_str("Console Successfully Initialized.\n");
        crate::KERNEL_CONSOLE.newline();
    };
    DEBUG_LINE.wait_for_connection();
    log::set_logger(&kernel_logger::KERNEL_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    alloc_impl::KERNEL_ALLOCATOR
        .initialize()
        .expect("Failed to initialized Global Allocator.");
    if let Some(kernel_addr) = KERNEL_ADDRESS_REQUEST.get_response().get() {
        unsafe {
            crate::KERNEL_CONSOLE.write_str("KERNEL BASE: ( virtual: 0x");
            crate::KERNEL_CONSOLE
                .write_str(
                    VirtualAddress::new(kernel_addr.virtual_base as usize).as_str(),
                );
            crate::KERNEL_CONSOLE.write_str(", physical: 0x");
            crate::KERNEL_CONSOLE
                .write_str(
                    PhysicalAddress::new(kernel_addr.physical_base as usize).as_str(),
                );
            crate::KERNEL_CONSOLE.write_str(" )");
            crate::KERNEL_CONSOLE.write_str("\n");
            crate::KERNEL_CONSOLE.newline();
        };
    }
    SYSTEM_IDT[0x80 as usize]
        .set_handler_fn(__irq_handler)
        .set_present(true)
        .set_privilege_level(x86_64::PrivilegeLevel::Ring0);
    SYSTEM_IDT[0x20].set_handler_fn(handle_pit).set_present(true);
    {
        /// This constant is used to avoid spamming the same compilation error ~200 times
        /// when the handler's signature is wrong.
        /// If we just passed `$handler` to `set_general_handler_recursive_bits`
        /// an error would be reported for every interrupt handler that tried to call it.
        /// With `GENERAL_HANDLER` the error is only reported once for this constant.
        const GENERAL_HANDLER: ::x86_64::structures::idt::GeneralHandlerFunc = __generic_error_irq_handler;
        {
            fn set_general_handler(
                idt: &mut ::x86_64::structures::idt::InterruptDescriptorTable,
                range: impl ::core::ops::RangeBounds<u8>,
            ) {
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) -> ! {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                                ::core::panicking::panic(
                                    "General handler returned on double fault",
                                );
                            }
                            idt.double_fault.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.invalid_tss.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.segment_not_present.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.stack_segment_fault.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.general_protection_fault.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: ::x86_64::structures::idt::PageFaultErrorCode,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code.bits()));
                            }
                            idt.page_fault.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.alignment_check.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) -> ! {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                                ::core::panicking::panic(
                                    "General handler returned on machine check exception",
                                );
                            }
                            idt.machine_check.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.cp_protection_exception.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        extern "x86-interrupt" fn handler(
                            frame: ::x86_64::structures::idt::InterruptStackFrame,
                            error_code: u64,
                        ) {
                            GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                        }
                        idt.vmm_communication_exception.set_handler_fn(handler);
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                                error_code: u64,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), Some(error_code));
                            }
                            idt.security_exception.set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (0 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (0 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (0 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (0 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (0 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (0 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (0 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 0 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
                {
                    const IDX: u8 = 1 | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4)
                        | (1 << 5) | (1 << 6) | (1 << 7);
                    #[allow(unreachable_code)]
                    if range.contains(&IDX) {
                        {
                            extern "x86-interrupt" fn handler(
                                frame: ::x86_64::structures::idt::InterruptStackFrame,
                            ) {
                                GENERAL_HANDLER(frame, IDX.into(), None);
                            }
                            idt[IDX as usize].set_handler_fn(handler);
                        };
                    }
                };
            }
            set_general_handler(&mut SYSTEM_IDT, 0..28);
        }
    };
    SYSTEM_IDT.load();
    legacy_pic::PRIMARY_PIC.enable(legacy_pic::Interrupt::PIT);
    legacy_pic::PRIMARY_PIC.sync();
    unsafe {
        crate::KERNEL_CONSOLE
            .write_str("Successfully loaded Interrupt Descriptor Table.");
        crate::KERNEL_CONSOLE.newline();
    };
    if let Some(module_response) = MODULE_REQUEST.get_response().get() {
        unsafe {
            crate::KERNEL_CONSOLE.write_str("MODULE COUNT: ");
            crate::KERNEL_CONSOLE
                .write_str(integer_to_string(module_response.module_count));
            crate::KERNEL_CONSOLE.write_str("\n");
            crate::KERNEL_CONSOLE.newline();
        };
        for module in module_response.modules() {
            let path = module
                .path
                .to_str()
                .expect("Failed to get Module Path")
                .to_str()
                .unwrap();
            let cmdline = module
                .cmdline
                .to_str()
                .expect("Failed to get Module Path")
                .to_str()
                .unwrap();
            let addr = module.base.as_ptr().unwrap() as usize;
            unsafe {
                crate::KERNEL_CONSOLE.write_str("$BOOT$");
                crate::KERNEL_CONSOLE.write_str(path);
                crate::KERNEL_CONSOLE
                    .write_str(": Successfully loaded... {\n parameters = [ ");
                crate::KERNEL_CONSOLE.write_str(cmdline);
                crate::KERNEL_CONSOLE.write_str(" ],\n base = 0x");
                crate::KERNEL_CONSOLE.write_str(VirtualAddress::new(addr).as_str());
                crate::KERNEL_CONSOLE.write_str("\n}");
                crate::KERNEL_CONSOLE.newline();
            };
            'module: {
                if path.contains(".TTF") {
                    let face_result = ttf_parser::Face::parse(
                        core::slice::from_raw_parts(
                            module.base.as_ptr().unwrap(),
                            module.length as usize,
                        ),
                        0,
                    );
                    if let Ok(face) = face_result {
                        let id = face.glyph_index('A').unwrap();
                        let mut builder = FontOutline::new();
                        face.outline_glyph(id, &mut builder).unwrap();
                        unsafe {
                            crate::KERNEL_CONSOLE.write_str("Font has ");
                            crate::KERNEL_CONSOLE
                                .write_str(integer_to_string(builder.segments().len()));
                            crate::KERNEL_CONSOLE.write_str(" Segments!");
                            crate::KERNEL_CONSOLE.newline();
                        };
                    }
                } else if path.contains(".SYS") {
                    let program_base = module.base.as_ptr().expect("No Module Base");
                    let program = core::slice::from_raw_parts(
                        program_base,
                        module.length as usize,
                    );
                    let bytes = match elf::ElfBytes::<
                        AnyEndian,
                    >::minimal_parse(program) {
                        Err(e) => {
                            {
                                let lvl = ::log::Level::Error;
                                if lvl <= ::log::STATIC_MAX_LEVEL
                                    && lvl <= ::log::max_level()
                                {
                                    ::log::__private_api::log(
                                        format_args!("Error Parsing Program: {0:#?}", e),
                                        lvl,
                                        &(
                                            "antos_kernel_minimal_generic",
                                            "antos_kernel_minimal_generic",
                                            "src/main.rs",
                                        ),
                                        530u32,
                                        ::log::__private_api::Option::None,
                                    );
                                }
                            };
                            break 'module;
                        }
                        Ok(v) => v,
                    };
                    KERNEL_PAGE_MAPPER
                        .lock()
                        .update_flags(
                            HugePage::from_start_address(
                                    VirtAddr::new(program_base.addr().transmute())
                                        .align_down(HugePage::SIZE),
                                )
                                .unwrap(),
                            PageTableFlags::PRESENT | !PageTableFlags::NO_EXECUTE,
                        );
                    let start: *const unsafe extern "C" fn(KernelProgramMeta) = core::mem::transmute(
                        program_base.addr() as u64 + bytes.ehdr.e_entry,
                    );
                }
            }
        }
    }
    INTERRUPT_HANDLERS[0x1] = Some(example_interrupt_handler);
    asm!("mov al, 0x1", options(preserves_flags, raw));
    {
        asm!("int {0}", const 0x80, options(nomem, nostack));
    };
    let mut result: u16;
    asm!("nop", out("rdx") result);
    {
        let lvl = ::log::Level::Info;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("Hello from the AntOS Kernel! ;)"),
                lvl,
                &(
                    "antos_kernel_minimal_generic",
                    "antos_kernel_minimal_generic",
                    "src/main.rs",
                ),
                572u32,
                ::log::__private_api::Option::None,
            );
        }
    };
    DEBUG_LINE.unsafe_write_line("End of Runtime.");
    loop {
        asm!("hlt");
    }
}
pub fn boolean_to_str(value: bool) -> &'static str {
    match value {
        true => "true",
        false => "false",
    }
}
#[cfg(not(test))]
#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        DEBUG_LINE
            .unsafe_write_line(
                _info
                    .payload()
                    .downcast_ref::<&'_ str>()
                    .unwrap_or(&"Panic!" as &&'_ str),
            );
    }
    {
        let lvl = ::log::Level::Error;
        if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
            ::log::__private_api::log(
                format_args!("A Rust Panic has occurred: \n{0:#?}\n", _info),
                lvl,
                &(
                    "antos_kernel_minimal_generic",
                    "antos_kernel_minimal_generic",
                    "src/main.rs",
                ),
                614u32,
                ::log::__private_api::Option::None,
            );
        }
    };
    hcf();
}
fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}
