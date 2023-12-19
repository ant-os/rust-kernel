#![allow(internal_features)]
#![feature(lang_items, start, rustc_private)]
#![no_std]
#![no_main]


pub macro __kernel_call($func:literal, $param_count:literal, $params:expr $return_storage:ident, $response_storage:ident){
    ::core::arch::asm!(
        "int 0x80",
        in("eax") $func,
        in("ecx") $param_count
        in("edx") $params,
        lateout("eax") $return_storage,
        lateout("ecx") $response_storage
    );
}

use core::arch::asm;
use core::panic::PanicInfo;

type DbgPrintFn = unsafe extern "C" fn (*const u8, usize);

#[repr(C)]
pub struct KernelProgramMeta{
    _dbg_print: *const DbgPrintFn
}

#[no_mangle]
#[start]
pub unsafe fn _start(__meta: KernelProgramMeta){
    let msg = "Hello from a Kernel Program!";
    (__meta._dbg_print)(msg.as_bytes().as_ptr(), msg.len());

    terminate_kernel_program_internal(0);
}

#[panic_handler]
pub unsafe fn __panic_handler(__info: &'_ PanicInfo<'_>) -> !{
    asm!(
        "mov ax, 0xA2",
        "int 0xA2",
        in("rdi") __info
    );

    loop {}
}