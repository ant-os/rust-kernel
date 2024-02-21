#![allow(internal_features)]
#![feature(lang_items, start, rustc_private)]
#![no_std]
#![no_main]

use core::arch::asm;
use core::panic::PanicInfo;

#[repr(C)]
pub struct DriverObject {}

pub unsafe fn kprint(msg: &'_ str) {
    let ptr = msg.as_ptr();
    let len = msg.len();

    asm!("int 0x80", in("r8w") ptr, in("r9w") len, options(nostack));
}

#[no_mangle]
#[start]
pub unsafe fn _start(__driver_object: *mut DriverObject) {}

#[panic_handler]
pub unsafe fn __panic_handler(__info: &'_ PanicInfo<'_>) -> ! {

    loop {}
}
