#![no_std]
#![no_main]
#![allow(deprecated)]
#![feature(panic_info_message)]
#![feature(unboxed_closures)]
#![feature(core_intrinsics)]
#![feature(decl_macro)]
#![feature(ptr_from_ref)]
#![feature(inherent_associated_types)]
#![feature(adt_const_params)]

pub mod bitmap_font;
pub mod common;
pub mod device;
pub mod framebuffer;
pub mod paging;
pub mod renderer;
pub mod serial;
pub mod status;

use crate::common::*;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use core::arch::asm;
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use crate::paging::pf_allocator;

static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);

decl_uninit! {
    PAGE_FRAME_ALLOCATOR => crate::paging::frame_allocator::PageFrameAllocator,
    ITOA_BUFFER => itoa::Buffer,
    PRIMARY_FRAMEBUFFER => crate::framebuffer::Framebuffer
}

pub const FONT_BITMAP: bitmap_font::BitmapFont = include!("bitmap.raw");
pub const DEBUG_LINE: serial::Port = serial::Port::COM1;

pub fn integer_to_string<'_str, I: itoa::Integer>(value: I) -> &'_str str {
    let mut buf = unsafe { &mut ITOA_BUFFER };

    assign_uninit! { ITOA_BUFFER (itoa::Buffer) <= { itoa::Buffer::new() } }

    unsafe { (*(*buf).as_mut_ptr()).format::<I>(value) }
}

struct Person {
    id: i32,
    age: i32,
    name: &'static str,
}

#[no_mangle]
unsafe extern "C" fn _start<'_kernel>() -> ! {
    DEBUG_LINE.wait_for_connection();

    // let formated = buffer.format(123);

    let mut total_memory = 0;

    if let Some(memmap_response) = MEMMAP_REQUEST.get_response().get() {
        DEBUG_LINE.unsafe_write_line("Got Memory Map Response");

        // We use [core::intrinsics::likely] to signal to the compiler that this condition is likely to be true.
        // Used for Optimization of the code!
        if core::intrinsics::likely(memmap_response.entry_count > 0) {
            for (index, entry) in memmap_response.memmap().iter().enumerate() {
                DEBUG_LINE.unsafe_write_string("MemoryMapEntry { index = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(index));
                DEBUG_LINE.unsafe_write_string(", base = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(entry.base));
                DEBUG_LINE.unsafe_write_string(", length = ");
                DEBUG_LINE.unsafe_write_string(integer_to_string(entry.len));
                DEBUG_LINE.unsafe_write_string(", type = \"");
                DEBUG_LINE.unsafe_write_string(match entry.typ {
                    MemoryMapEntryType::Usable => "Usable",
                    MemoryMapEntryType::Reserved => "Reserved",
                    MemoryMapEntryType::KernelAndModules | MemoryMapEntryType::Framebuffer => {
                        "Kernel/Framebuffer"
                    }
                    _ => "Other",
                });
                DEBUG_LINE.unsafe_write_string("\", }\n\r");

                total_memory += entry.len;
            }
            assign_uninit! { PAGE_FRAME_ALLOCATOR (paging::frame_allocator::PageFrameAllocator) <= { paging::frame_allocator::PageFrameAllocator::from_response(memmap_response) }};
        } else {
            DEBUG_LINE.unsafe_write_line("No Entries in Memory Map!")
        }
    } else {
        DEBUG_LINE.unsafe_write_line("Failed to get Memory Map!");
    }

    DEBUG_LINE.unsafe_write_line("TEST");

    let mut page = pf_allocator!().request_safe_page();

    let page_as_person: &mut Person = &mut *page.unchecked_raw_transmute::<Person>();

    debug!("TEST");

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
    unsafe {
        DEBUG_LINE.unsafe_write_line(
            _info
                .payload()
                .downcast_ref::<&'_ str>()
                .unwrap_or(&"Panic!" as &&'_ str),
        );
    }

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
