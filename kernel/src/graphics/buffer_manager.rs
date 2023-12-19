use core::{sync::atomic::AtomicBool, alloc::Layout};


use alloc::sync::Arc;
use spin::RwLock;

use super::buffer::{AtomicBuffer, Buffer};


#[derive(Debug)]
pub struct BufferManager{
    front: Arc<AtomicBuffer>,
    back: Arc<AtomicBuffer>
}

impl BufferManager{
    pub fn new(front_buffer: Arc<AtomicBuffer>) -> Self{
        let front = front_buffer.as_ref().read();

        let back = AtomicBuffer::new( Buffer {
            base: unsafe { alloc::alloc::alloc_zeroed(Layout::for_value(&front)) },
            offset: (0,0),
            height: front.height,
            width: front.width,
        });

        Self { front: front_buffer, back: Arc::new(back) }
    }
}