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
        outb(self.get_addr() + 1, 0x00); // Disable all interrupts
        outb(self.get_addr() + 3, 0x80); // Enable DLAB (set baud rate divisor)
        outb(self.get_addr() + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        outb(self.get_addr() + 1, 0x00); //                  (hi byte)
        outb(self.get_addr() + 3, 0x03); // 8 bits, no parity, one stop bit
        outb(self.get_addr() + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        outb(self.get_addr() + 4, 0x0B); // IRQs enabled, RTS/DSR set
        outb(self.get_addr() + 4, 0x1E); // Set in loopback mode, test the serial chip
        outb(self.get_addr() + 0, 0xAE); // Test serial chip (send byte 0xAE and check if serial returns same byte)

        // Check if serial is faulty (i.e: not same byte as sent)
        if (inb(self.get_addr() + 0) != 0xAE) {
            return true;
        }

        // If serial is not faulty set it in normal operation mode
        // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
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
        unimplemented!();
    }
}

