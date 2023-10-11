#![no_std]
#![no_main]
#![allow(deprecated)]
#![feature(panic_info_message)]
#![feature(unboxed_closures)]


pub mod bitmap_font;
pub mod common;
pub mod device;
pub mod framebuffer;
pub mod renderer;
pub mod serial;
pub mod status;

use crate::device::{character::{*, TimedCharacterDevice}, Device, GeneralDevice};
use core::arch::asm;
#[macro_use] use core::fmt::*;

static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);

const FONT_BITMAP: bitmap_font::BitmapFont = include!("bitmap.raw");
const DEBUG_LINE: serial::Port = serial::Port::COM1;

#[no_mangle]
unsafe extern "C" fn _start() -> ! {

    DEBUG_LINE.wait_for_connection();

    hcf();
}

macro_rules! halt_while {
    ($boolean_expr:expr) => {
        while ($boolean_expr) {
            asm!("hlt");
        }
    };
}

#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {

    unsafe {DEBUG_LINE.unsafe_write_line(_info.payload().downcast_ref::<&'_ str>().unwrap_or(&"Panic!" as &&'_ str));}

    hcf();
}
fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}
