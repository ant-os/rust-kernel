//! Legacy Programmable Interrupt Controller.

const PIC1_COMMAND: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;

const PIC2_COMMAND: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_EOI: u8 = 0x20;

const ICW1_INIT: u8 = 0x10;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;

use bit::BitIndex;

use super::io::{inb, io_wait, outb};

pub struct PIC {
    pub(self) master_mask: u8,
    pub(self) slave_mask: u8,
}

#[derive(Debug)]
pub enum Interrupt {
    PIT,
}

impl Interrupt {
    pub fn enable_in(self, pic: &'_ mut PIC, value: bool) {
        match self {
            Interrupt::PIT => pic.master_mask.set_bit(0, !value),
            _ => todo!(),
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
        if !self.needs_sync() {
            return;
        }

        self.remap();
        outb(PIC1_DATA, self.master_mask);
        outb(PIC2_DATA, self.slave_mask);
    }

    pub fn enable(&mut self, int: Interrupt) {
        int.enable_in(self, true)
    }

    pub fn disable(&mut self, int: Interrupt) {
        int.enable_in(self, false)
    }
}

pub(crate) static mut PRIMARY_PIC: PIC = PIC::new();
