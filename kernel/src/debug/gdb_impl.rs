use super::protocol::{Protocol, ProtocolImpl};
use alloc::boxed::Box;

pub struct Gdb;

impl ProtocolImpl for Gdb{
    fn handle_packet(packet: &[u8]) -> Box<[u8]>{
        
    }
}