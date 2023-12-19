use core::{cell::UnsafeCell, default};
use core::marker;
use core::sync::atomic::AtomicPtr;

use alloc::sync::Arc;
use limine::NonNullPtr;
use lock_api::{RwLock, GuardSend};

#[marker]
pub trait Handler{}

pub fn get_data_for_handler(handle: Handle) -> Option<*mut dyn Handler>{
    None
}

#[repr(transparent)]
pub struct Handle{
    pub(self) inner: u64
}

impl Handle{
    pub fn new(offset: u64) -> Self{
        Self { inner: offset }
    }
}
