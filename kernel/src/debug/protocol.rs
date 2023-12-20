//! GDB Serial Remote Protocol
//! =========================
//! This module implements the GDB Serial Remote Protocol, which is used to
//! communicate with the kernel over a serial port.
//!   https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html

use core::marker::PhantomData;

use x86_64::structures::idt::{ExceptionVector, InterruptStackFrame};

use crate::{serial::Port as SerialPort, device::character::UnsafeCharacterDevice};
use alloc::boxed::Box;

const STATIC_PACKAGE_BUFFER_SIZE: usize = 4096;

static mut PACKET_BUFFER: [u8; STATIC_PACKAGE_BUFFER_SIZE] = [0u8; STATIC_PACKAGE_BUFFER_SIZE];

/// GDB Serial Remote Protocol.
/// ===========================
/// This struct is thread-safe.
/// 
/// It is safe to use this struct from multiple CPUs at the same time.
/// This is because the serial port is only accessed while handling a breakpoint exception.
/// The breakpoint exception handler is only called by one CPU at a time.
pub struct Protocol<I: ProtocolImpl + 'static> {
    /// Serial port to use for communication.
    serial_port: spin::RwLock<SerialPort>,
    /// Buffer for incoming packets.
    /// This is a static buffer because we maybe be unable to allocate memory while handling
    /// a breakpoint exception.
    /// The buffer is only used while handling a breakpoint exception, so it's
    /// fine to use a static buffer.
    /// TODO: Make this more thread-safe?
    packet_buffer: &'static mut [u8],
    
    /// Protocol implementation.
    /// This is a phantom data field because we only need the type information.
    _protocol_impl: PhantomData<I>
}

/// Protocol implementation.
/// Functions in this trait are called when the corresponding packet is received.
pub trait ProtocolImpl where Self: 'static {
    /// Handle a packet.
    /// The packet is guaranteed to be valid.
    /// The response must be a valid packet.
    fn handle_packet(packet: &[u8]) -> Box<[u8]>;
}

// Mark the Protocol as being thread-safe.
// This is safe because the serial port is only accessed while handling a breakpoint exception.
// The breakpoint exception handler is only called by one CPU at a time.
unsafe impl<I: ProtocolImpl + 'static> Sync for Protocol<I> { /* empty */ }
unsafe impl<I: ProtocolImpl + 'static> Send for Protocol<I> { /* empty */ }

impl<I: ProtocolImpl + 'static> Protocol<I>{
    pub fn new(serial_port: SerialPort) -> Self{
        Self{
            serial_port: spin::RwLock::new(serial_port),
            packet_buffer: unsafe { &mut PACKET_BUFFER },
            _protocol_impl: PhantomData
        }
    }

    pub fn handle_exception(&mut self, exception: ExceptionVector, stack_frame: InterruptStackFrame){
        if exception == ExceptionVector::Breakpoint{
            self.handle_packet();
        }
    }

    pub fn handle_serial_interrupt(&mut self){
        todo!("Handle serial interrupt.");
    }

    fn handle_packet(&mut self){
        // Read the packet from the serial port.
        let packet = self.read_packet();

        // Handle the packet.
        let response = I::handle_packet(packet);

        // Send the response.
        self.send_packet(response);
    }

    pub fn update(&mut self){
        self.handle_packet();
    }

    fn read_packet(&mut self) -> &[u8]{
        let _start_byte = unsafe { self.serial_port.read().read_raw() };

        // Read the packet until the end character.
        let mut packet_length = 0;
        loop{
            let byte = unsafe { self.serial_port.read().read_raw() };
            if byte == b'#'{
                break;
            }

            if packet_length == self.packet_buffer.len(){
                unsafe { self.serial_port.read().write_raw('-' as u8) };
                return &[];
            }

            self.packet_buffer[packet_length] = byte;
            packet_length += 1;
        }

        let checksum = unsafe { self.serial_port.read().read_raw() };

        let mut expected_checksum: u8 = 0;
        for i in 0..packet_length{
            expected_checksum = expected_checksum.wrapping_add(self.packet_buffer[i] as u8);
        }
        expected_checksum &= 0xff;

        if checksum != expected_checksum{
            unsafe { self.serial_port.read().write_raw('-' as u8) };
            return &[];
        }

        // Send an acknowledgement.
        unsafe { self.serial_port.read().write_raw('+' as u8) };

        &self.packet_buffer[..packet_length]
    }

    fn send_packet(&mut self, packet: Box<[u8]>) {
        let mut checksum: u8 = 0;
        for i in 0..packet.len() {
            checksum = checksum.wrapping_add(packet[i] as u8);
        }
        checksum &= 0xff;

        unsafe { self.serial_port.read().write_raw('$' as u8) };

        for i in 0..packet.len() {
            unsafe { self.serial_port.read().write_raw(packet[i]) };
        }

        unsafe { self.serial_port.read().write_raw(checksum) };
    }
}
