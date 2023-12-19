//! Interrupt Descriptor Table

use core::{arch::asm, alloc::Layout, sync::atomic::AtomicPtr};
use bitflags::bitflags;

use crate::{alloc_impl::KERNEL_ALLOCATOR, PAGE_TABLE_MANAGER, paging::pt_manager, memory::{VirtualAddress, PhysicalAddress}};

const GDT_KERNEL_CODE: u16 = 0x8;

pub type IdtEntries = [IdtEntry; 256];

#[repr(C)]
#[derive(Debug)]
pub struct Idt {
    pub entries: IdtEntries
}

impl Idt {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::new(); 256]
        }
    }

    pub unsafe fn load_into(&self) {
        super::lidt(&self.pointer())
    }

    pub fn pointer(&self) -> super::DescriptorTablePointer{
        use core::mem::size_of;
        super::DescriptorTablePointer {
            base: self as *const _ as u64,
            limit: (size_of::<Self>() - 1) as u16,
        }
    }
}

bitflags! {
    pub struct IdtFlags: u8 {
        const PRESENT = 1 << 7;
        const RING_0 = 0 << 5;
        const RING_1 = 1 << 5;
        const RING_2 = 2 << 5;
        const RING_3 = 3 << 5;
        const SS = 1 << 4;
        const INTERRUPT = 0xE;
        const TRAP = 0xF;
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[repr(packed)]
pub struct IdtEntry {
    offsetl: u16,
    selector: u16,
    zero: u8,
    attribute: u8,
    offsetm: u16,
    offseth: u32,
    _zero2: u32
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
            _zero2: 0
        }
    }

    pub fn set_flags(&mut self, flags: IdtFlags) {
        self.attribute = flags.bits();
    }

    pub fn set_ist(&mut self, ist: u8) {
        assert_eq!(ist & 0x07, ist, "interrupt stack table must be within 0..=7");
        self.zero &= 0xF8;
        self.zero |= ist;
    }

    pub fn set_offset(&mut self, selector: u16, base: usize) {
        self.selector = selector;
        self.offsetl = base as u16;
        self.offsetm = (base >> 16) as u16;
        self.offseth = ((base as u64) >> 32) as u32;
    }

    // A function to set the offset more easily
    pub fn set_func(&mut self, func: unsafe extern fn()) {
        self.set_flags(IdtFlags::PRESENT | IdtFlags::RING_0 | IdtFlags::INTERRUPT);
        self.set_offset(0x8, func as usize);
    }
}

lazy_static::lazy_static!{
    pub static ref KERNEL_IDT: AtomicPtr<Idt> = unsafe {
        KERNEL_ALLOCATOR.assume_init();
        
        let idt = alloc::boxed::Box::<Idt>::into_raw(alloc::boxed::Box::<Idt>::new(Idt::new()));

       //  pt_manager!().map_memory(VirtualAddress::new(idt.addr()), PhysicalAddress::new(idt.addr()));

        AtomicPtr::new(idt)
    };
}