use spin::RwLock;

use super::Color;

#[derive(Debug, Default, Clone, Copy)]
pub struct Buffer{
    pub base: *mut Color,
    pub offset: (u64, u64),
    pub height: u64,
    pub width: u64,
}

pub type AtomicBuffer = RwLock<Buffer>;