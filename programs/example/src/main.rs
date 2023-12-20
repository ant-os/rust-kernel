#![allow(internal_features)]
#![feature(lang_items, start, rustc_private)]
#![no_std]
#![no_main]

#[inline(never)]
#[no_mangle]
#[link_section = ".kernelcall"]
#[export_name = "__kernelcall_print"]
pub unsafe extern "C" fn print(buffer: *const u8, len: usize) -> u64 {
    let mut status: u64;
    ::core::arch::asm!(
        "mov r8, 0x6",
        "int 0x80",
        in("r9") buffer,
        in("r10") len,
        lateout("r8") status,
    );
    status
}

use core::panic::PanicInfo;


#[no_mangle]
#[start]
pub unsafe fn _start(){
    let msg = "Hello from a Kernel Program!";
    print(msg.as_bytes().as_ptr(), msg.len());
}

#[panic_handler]
pub unsafe fn __panic_handler(__info: &'_ PanicInfo<'_>) -> !{
    print("Kernel Panic!\0".as_ptr(), 14);
    loop{}
}