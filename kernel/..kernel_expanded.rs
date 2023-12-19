#![feature(prelude_import)]
#![no_std]
#![no_main]
#![allow(deprecated)]
#![feature(panic_info_message)]
#![feature(unboxed_closures)]
#![feature(core_intrinsics)]
#![feature(decl_macro)]
#![feature(ptr_from_ref)]
#![feature(inherent_associated_types)]
#![feature(adt_const_params)]
#![feature(abi_x86_interrupt)]
#![recursion_limit = "2000"]
#[prelude_import]
use core::prelude::rust_2021::*;
#[macro_use]
extern crate core;
extern crate compiler_builtins as _;
pub mod bitmap_font {
    pub type BitmapChar = [u16; 8];
    pub type BitmapFont = [BitmapChar; 128];
    pub trait BitmapCharImpl {
        fn is_set(&self, x: usize, y: usize) -> bool;
    }
    impl BitmapCharImpl for BitmapChar {
        fn is_set(&self, x: usize, y: usize) -> bool {
            (self[x] & 1 << (y as i8)) != 0
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
        pub unsafe fn io_wait() {
            ::core::panicking::panic("not implemented");
        }
    }
    pub mod macros {}
    pub use limine::*;
    pub mod idt {
        //! Interrupt Descriptor Table
        use core::arch::asm;
        use bitflags::bitflags;
        const GDT_KERNEL_CODE: u16 = 1;
        pub type IdtEntries = [IdtEntry; 256];
        #[repr(C)]
        pub struct Idt {
            pub entries: IdtEntries,
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
                self.set_offset((GDT_KERNEL_CODE as u16) << 3, func as usize);
            }
        }
    }
    pub mod gdt {
        //! Global Descriptor Table
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
        pub(crate) static mut INIT_GDT: [GdtEntry; 3] = [
            GdtEntry::new(0, 0, 0, 0),
            GdtEntry::new(
                0,
                0,
                GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_EXECUTABLE
                    | GDT_A_PRIVILEGE,
                GDT_F_LONG_MODE,
            ),
            GdtEntry::new(
                0,
                0,
                GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_PRIVILEGE,
                GDT_F_LONG_MODE,
            ),
        ];
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
    pub type Unit = ();
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
            asm!(
                "lidt [{0}]", in (reg) ptr, options(readonly, preserves_flags, nostack)
            );
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
            >(
                (unsafe { &mut *crate::PAGE_FRAME_ALLOCATOR.as_mut_ptr() })
                    .request_page()? as *mut (),
            )
        })
    }
    pub macro endl {
        () => { "\n\r" }
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
pub mod paging {
    use bit::BitIndex;
    use crate::memory::PhysicalAddress;
    pub mod frame_allocator {
        use crate::{
            consts::{INVALID_PAGE_STATE, PAGE_SIZE},
            debug, endl, extra_features,
        };
        use limine::{MemmapEntry, MemmapResponse, MemoryMapEntryType, NonNullPtr};
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
                    bitmap_segment.len as usize / (PAGE_SIZE + 1) as usize,
                );
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
        pub struct PtrWrapper<T: ?Sized>(*mut T);
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
    }
    pub mod table_manager {
        use core::{ptr::NonNull, char::UNICODE_VERSION, f64::consts::E};
        use limine::NonNullPtr;
        use crate::{common::_alloc_frame_as_mut_t, debug};
        use super::{PageTable, indexer::PageMapIndexer, PageTableEntry};
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
        pub(self) macro _page_table {
            ($table : expr, $index : expr) => { { let mut entry = (& mut (& mut *
            ($table).as_ptr()).entries[$index]); let result : NonNullPtr < PageTable > =
            if ! entry.present() { let new_table = core::mem::transmute:: < NonNull <
            PageTable >, NonNullPtr < PageTable >> (NonNull:: < PageTable >
            ::new(_alloc_frame_as_mut_t:: < PageTable > ().map_err(| err |
            MemoryMapError::FrameAllocator(err)) ?)
            .ok_or(MemoryMapError::FrameAllocator(super::frame_allocator::Error::OutOfMemory))
            ?); entry.set_addr(new_table.as_ptr() as usize); entry.set_present(true);
            entry.set_rw(true); new_table } else {
            core::mem::transmute(NonNull::new(core::mem::transmute:: < usize, * mut
            PageTable > (entry.addr().data())).ok_or(MemoryMapError::TableNotFound) ?) };
            result } }
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
            /// Internal Function for Mapping Memory.
            pub(crate) unsafe fn map_memory_internal(
                &self,
                virtual_addr: VirtualAddress,
                physical_addr: PhysicalAddress,
            ) -> Result<(), MemoryMapError> {
                let indexer = PageMapIndexer::for_addr(virtual_addr.data());
                let pdp = {
                    let mut entry = (&mut (&mut *(self.PML4).as_ptr())
                        .entries[indexer.pdp]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let pd = {
                    let mut entry = (&mut (&mut *(pdp).as_ptr()).entries[indexer.pd]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let pt = {
                    let mut entry = (&mut (&mut *(pd).as_ptr()).entries[indexer.pt]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let mut entry = &mut (&mut *pt.as_ptr()).entries[indexer.p];
                entry.set_addr(physical_addr.data());
                entry.set_present(true);
                entry.set_rw(true);
                Ok(())
            }
            /// Internal Function for Mapping Memory.
            pub(crate) unsafe fn get_page_entry(
                &self,
                virtual_addr: VirtualAddress,
            ) -> Result<usize, MemoryMapError> {
                let indexer = PageMapIndexer::for_addr(virtual_addr.data());
                let pdp = {
                    let mut entry = (&mut (&mut *(self.PML4).as_ptr())
                        .entries[indexer.pdp]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let pd = {
                    let mut entry = (&mut (&mut *(pdp).as_ptr()).entries[indexer.pd]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let pt = {
                    let mut entry = (&mut (&mut *(pd).as_ptr()).entries[indexer.pt]);
                    let result: NonNullPtr<PageTable> = if !entry.present() {
                        let new_table = core::mem::transmute::<
                            NonNull<PageTable>,
                            NonNullPtr<PageTable>,
                        >(
                            NonNull::<
                                PageTable,
                            >::new(
                                    _alloc_frame_as_mut_t::<PageTable>()
                                        .map_err(|err| MemoryMapError::FrameAllocator(err))?,
                                )
                                .ok_or(
                                    MemoryMapError::FrameAllocator(
                                        super::frame_allocator::Error::OutOfMemory,
                                    ),
                                )?,
                        );
                        entry.set_addr(new_table.as_ptr() as usize);
                        entry.set_present(true);
                        entry.set_rw(true);
                        new_table
                    } else {
                        core::mem::transmute(
                            NonNull::new(
                                    core::mem::transmute::<
                                        usize,
                                        *mut PageTable,
                                    >(entry.addr().data()),
                                )
                                .ok_or(MemoryMapError::TableNotFound)?,
                        )
                    };
                    result
                };
                let entry = &pt.entries[indexer.p];
                Ok(entry.data())
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
    pub macro pf_allocator {
        () => { (unsafe { & mut * crate ::PAGE_FRAME_ALLOCATOR.as_mut_ptr() }) }
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
            (unsafe { &mut *crate::PAGE_FRAME_ALLOCATOR.as_mut_ptr() })
                .free_page(self.0)
                .unwrap();
        }
    }
    impl Drop for SafePagePtr {
        fn drop(&mut self) {
            self.free();
        }
    }
    pub struct PageTableEntry(usize);
    impl PageTableEntry {
        pub fn present(&self) -> bool {
            self.0.bit(1)
        }
        pub fn set_present(&mut self, value: bool) {
            self.0.set_bit(0, value);
        }
        pub fn rw(&self) -> bool {
            self.0.bit(1)
        }
        pub fn set_rw(&mut self, value: bool) {
            self.0.set_bit(1, value);
        }
        pub fn addr(&self) -> PhysicalAddress {
            PhysicalAddress::new(self.0 & 0x0000_FFFF_FFFF_F000)
        }
        pub fn set_addr(&mut self, value: usize) {
            self.0.set_bit_range(12..64, value);
        }
        pub fn data(&self) -> usize {
            self.0
        }
    }
    #[repr(align(0x1000))]
    pub struct PageTable {
        pub entries: [PageTableEntry; 512],
    }
}
pub mod renderer {
    type Color = u32;
    use crate::bitmap_font::{BitmapCharImpl, BitmapFont};
    use crate::framebuffer::Framebuffer;
    pub struct Renderer {
        target_fb: &'static Framebuffer,
        foreground_color: Color,
        background_color: Color,
        bitmap_font: &'static BitmapFont,
    }
    pub enum RendererError {
        OutOfBounds,
    }
    impl Renderer {
        pub fn new(fb: &'static Framebuffer, font: &'static BitmapFont) -> Renderer {
            Self {
                target_fb: fb,
                foreground_color: 0xFFFFFFFF,
                background_color: 0x00000000,
                bitmap_font: font,
            }
        }
        pub unsafe fn unsafe_put_pixel(&self, x: usize, y: usize, color: Color) {
            let pixel_offset = x * (self.target_fb.pitch as usize + y);
            *(self.target_fb.address.as_ptr().unwrap().offset(pixel_offset as isize)
                as *mut Color) = color;
        }
        pub unsafe fn unsafe_pull_pixel(&self, x: usize, y: usize) -> Color {
            let pixel_offset = x * (self.target_fb.pitch as usize + y);
            *(self.target_fb.address.as_ptr().unwrap().offset(pixel_offset as isize)
                as *mut Color)
        }
        pub unsafe fn unsafe_draw_char(&self, off_x: usize, off_y: usize, chr: i8) {
            for x in 0..8 as usize {
                for y in 0..8 as usize {
                    self.unsafe_put_pixel(
                        off_x + x,
                        off_y + y,
                        if self.bitmap_font[chr as usize].is_set(x, y) {
                            self.foreground_color
                        } else {
                            self.background_color
                        },
                    );
                }
            }
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
                self.unsafe_write_string("(CONNECTED)\n\r");
            }
        }
        pub unsafe fn unsafe_write_string(&self, _str: &'static str) {
            for chr in _str.chars() {
                self.write(chr as u8);
            }
        }
        pub unsafe fn unsafe_write_line(&self, _str: &'static str) {
            self.unsafe_write_string(_str);
            self.unsafe_write_string("\n\r");
        }
        pub unsafe fn unsafe_read_string(&self, len: usize) -> &'static str {
            ::core::panicking::panic("not implemented");
        }
    }
}
pub mod status {}
pub mod memory {
    //! Common Memory Structures, e.g [VirtualAddress].
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
    }
    pub struct MemoryArea {
        pub base: PhysicalAddress,
        pub size: usize,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for MemoryArea {
        #[inline]
        fn clone(&self) -> MemoryArea {
            let _: ::core::clone::AssertParamIsClone<PhysicalAddress>;
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
}
use crate::common::*;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use crate::memory::{VirtualAddress, PhysicalAddress};
use crate::paging::table_manager::PageTableManager;
use core::arch::asm;
use core::ffi::CStr;
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use core::mem;
use crate::paging::{pf_allocator, PageTable};
#[macro_use]
extern crate bitfield;
static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static KERNEL_ADDRESS_REQUEST: limine::KernelAddressRequest = limine::KernelAddressRequest::new(
    0,
);
static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(
    0,
);
static MODULE_REQUEST: limine::ModuleRequest = limine::ModuleRequest::new(0);
static TEST1: &'static str = "Hello Paging!";
static TEST2: &'static str = "):";
static mut PAGE_FRAME_ALLOCATOR: core::mem::MaybeUninit<
    crate::paging::frame_allocator::PageFrameAllocator,
> = core::mem::MaybeUninit::uninit();
static mut ITOA_BUFFER: core::mem::MaybeUninit<itoa::Buffer> = core::mem::MaybeUninit::uninit();
static mut PRIMARY_FRAMEBUFFER: core::mem::MaybeUninit<
    crate::framebuffer::Framebuffer,
> = core::mem::MaybeUninit::uninit();
static mut KERNEL_CMDLINE: core::mem::MaybeUninit<&'static str> = core::mem::MaybeUninit::uninit();
static mut KERNEL_PATH: core::mem::MaybeUninit<&'static str> = core::mem::MaybeUninit::uninit();
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
pub unsafe extern "C" fn irq0_timer() {
    unsafe { DEBUG_LINE.unsafe_write_line("Tick!") }
}
#[no_mangle]
unsafe extern "C" fn _start<'_kernel>() -> ! {
    DEBUG_LINE.wait_for_connection();
    let mut total_memory = 0;
    if let Some(memmap_response) = MEMMAP_REQUEST.get_response().get() {
        DEBUG_LINE.unsafe_write_line("Got Memory Map Response");
        if core::intrinsics::likely(memmap_response.entry_count > 0) {
            for (index, entry) in memmap_response.memmap().iter().enumerate() {
                DEBUG_LINE.unsafe_write_string("MemoryMapEntry { index = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(index));
                DEBUG_LINE.unsafe_write_string(", base = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(entry.base));
                DEBUG_LINE.unsafe_write_string(", length = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(entry.len));
                DEBUG_LINE.unsafe_write_string(", type = \"");
                DEBUG_LINE
                    .unsafe_write_string(
                        match entry.typ {
                            MemoryMapEntryType::Usable => "Usable",
                            MemoryMapEntryType::Reserved => "Reserved",
                            MemoryMapEntryType::KernelAndModules
                            | MemoryMapEntryType::Framebuffer => "Kernel/Framebuffer",
                            _ => "Other",
                        },
                    );
                DEBUG_LINE.unsafe_write_string("\", }\n\r");
                total_memory += entry.len;
            }
            *(unsafe {
                &mut PAGE_FRAME_ALLOCATOR
            }) = core::mem::MaybeUninit::<
                paging::frame_allocator::PageFrameAllocator,
            >::new(
                ({
                    paging::frame_allocator::PageFrameAllocator::from_response(
                        memmap_response,
                    )
                }),
            );
        } else {
            DEBUG_LINE.unsafe_write_line("No Entries in Memory Map!")
        }
    } else {
        DEBUG_LINE.unsafe_write_line("Failed to get Memory Map!");
    }
    common::lgdt(
        &DescriptorTablePointer {
            limit: (gdt::INIT_GDT.len() * core::mem::size_of::<gdt::GdtEntry>() - 1)
                as u16,
            base: gdt::INIT_GDT.as_ptr() as u64,
        },
    );
    let page_table_manager = PageTableManager::new().expect("Failed to crate PTM.");
    if let Some(kernel_addr) = KERNEL_ADDRESS_REQUEST.get_response().get() {
        unsafe {
            crate::DEBUG_LINE.unsafe_write_string("KERNEL BASE: ");
            crate::DEBUG_LINE
                .unsafe_write_string(integer_to_string(kernel_addr.physical_base));
        };
    }
    if let Some(kernel_file) = KERNEL_FILE_REQUEST.get_response().get() {
        if let Some(file) = kernel_file.kernel_file.get() {
            *(unsafe {
                &mut KERNEL_CMDLINE
            }) = core::mem::MaybeUninit::<
                &'static str,
            >::new(
                (core::mem::transmute(
                    file
                        .cmdline
                        .to_str()
                        .expect("Failed to get kernel cmdline.")
                        .to_str()
                        .unwrap(),
                )),
            );
            *(unsafe {
                &mut KERNEL_PATH
            }) = core::mem::MaybeUninit::<
                &'static str,
            >::new(
                (core::mem::transmute(
                    file
                        .path
                        .to_str()
                        .expect("Failed to get kernel path.")
                        .to_str()
                        .unwrap(),
                )),
            );
        }
        unsafe {
            crate::DEBUG_LINE.unsafe_write_string("\n\r");
        };
        unsafe {
            crate::DEBUG_LINE.unsafe_write_string("CMDLINE: ");
            crate::DEBUG_LINE.unsafe_write_string(KERNEL_CMDLINE.assume_init());
            crate::DEBUG_LINE.unsafe_write_string("\n\r");
        };
        unsafe {
            crate::DEBUG_LINE.unsafe_write_string("PATH: ");
            crate::DEBUG_LINE.unsafe_write_string(KERNEL_PATH.assume_init());
            crate::DEBUG_LINE.unsafe_write_string("\n\r");
        };
    }
    if let Some(module_response) = MODULE_REQUEST.get_response().get() {
        unsafe {
            crate::DEBUG_LINE.unsafe_write_string("MODULE COUNT: ");
            crate::DEBUG_LINE
                .unsafe_write_string(integer_to_string(module_response.module_count));
            crate::DEBUG_LINE.unsafe_write_string("\n\r");
        };
        for module in module_response.modules() {
            unsafe {
                crate::DEBUG_LINE.unsafe_write_string("Found Module ");
                crate::DEBUG_LINE
                    .unsafe_write_string(
                        module
                            .path
                            .to_str()
                            .expect("Failed to get Module Path")
                            .to_str()
                            .unwrap(),
                    );
                crate::DEBUG_LINE.unsafe_write_string("!");
                crate::DEBUG_LINE.unsafe_write_string("\n\r");
            };
            let addr = module.base.as_ptr().unwrap() as usize;
            page_table_manager
                .map_memory_internal(
                    VirtualAddress::new(addr),
                    PhysicalAddress::new(addr),
                );
            unsafe {
                crate::DEBUG_LINE
                    .unsafe_write_string("Content(Null-Terminated String): ");
                crate::DEBUG_LINE
                    .unsafe_write_string(
                        CStr::from_ptr(
                                module
                                    .base
                                    .as_ptr()
                                    .map(|ptr| core::mem::transmute::<*mut _, *const _>(ptr))
                                    .expect("Failed to get Data of Module"),
                            )
                            .to_str()
                            .unwrap(),
                    );
                crate::DEBUG_LINE.unsafe_write_string("\n\r");
            };
        }
    }
    DEBUG_LINE.unsafe_write_line("End of Runtime.");
    hcf();
}
pub fn boolean_to_str(value: bool) -> &'static str {
    match value {
        true => "true",
        false => "false",
    }
}
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
