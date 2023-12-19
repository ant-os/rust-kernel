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
#![feature(abi_x86_interrupt)]
#![feature(strict_provenance)]
#![recursion_limit = "2000"]

pub mod bitmap_font;
pub mod common;
pub mod device;
pub mod framebuffer;
pub mod paging;
pub mod renderer;
pub mod serial;
pub mod status;
pub mod memory;
pub mod tty;

pub(crate) use tty::KERNEL_CONSOLE;
use crate::common::*;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use crate::memory::{VirtualAddress, PhysicalAddress};
use crate::paging::table_manager::PageTableManager;
use core::arch::asm;
use core::ffi::CStr;
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use core::mem;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
use crate::paging::{pf_allocator, PageTable, pt_manager};
#[macro_use] extern crate bitfield;
use numtoa::NumToA;
use renderer::Renderer;

static FRAMEBUFFERS_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static KERNEL_ADDRESS_REQUEST: limine::KernelAddressRequest = limine::KernelAddressRequest::new(0);
static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(0);
static MODULE_REQUEST: limine::ModuleRequest = limine::ModuleRequest::new(0);

pub(crate) static mut GENERIC_STATIC_BUFFER: [u8; 25] = [0u8; 25];

static TEST1: &'static str = "Hello Paging!";

static TEST2: &'static str = "):";

decl_uninit! {
    PAGE_FRAME_ALLOCATOR => crate::paging::frame_allocator::PageFrameAllocator,
    ITOA_BUFFER => itoa::Buffer,
    KERNEL_CMDLINE => &'static str,
    KERNEL_PATH => &'static str,
    PAGE_TABLE_MANAGER => crate::paging::table_manager::PageTableManager
}

lazy_static::lazy_static!{
    pub(crate) static ref FRAMEBUFFERS: &'static [NonNullPtr<limine::Framebuffer>] = {
        if let Some(fb_resp) = FRAMEBUFFERS_REQUEST.get_response().get(){
            fb_resp.framebuffers::<'static>()
        }else{
            debug_err!("Failed to get the list of System Framebuffers!");
            panic!("Failed to get the list of System Framebuffers!");
        }
    };

   
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

pub unsafe extern "C" fn irq0_timer() {
    unsafe { DEBUG_LINE.unsafe_write_line("Tick!") }
}

#[no_mangle]
unsafe extern "C" fn _start<'_kernel>() -> ! {
    DEBUG_LINE.wait_for_connection();

    // let formated = buffer.format(123);

    let mut kernel_renderer = renderer::Renderer::new(FRAMEBUFFERS.get(0).expect("No System Framebuffers."), &FONT_BITMAP);

    kernel_renderer.update_colors(Some(0xFFFFFFFF), Some(0xFF010FFF));
    kernel_renderer.clear(0xFF010FFF);
    kernel_renderer.optional_font_scaling = Some(2);

    Renderer::make_global(kernel_renderer);

    KERNEL_CONSOLE.newline();

    kprint!("Hello World!\n");

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

    common::lgdt(&DescriptorTablePointer {
        limit: (gdt::INIT_GDT.len() * core::mem::size_of::<gdt::GdtEntry>() - 1) as u16,
        base: gdt::INIT_GDT.as_ptr() as u64,
    });

    let mut pml4_addr = 0usize;

    asm!("mov cr0, {0}", out(reg) pml4_addr);

    let pml4 = NonNull::<PageTable>::dangling().with_addr(NonZeroUsize::new(pml4_addr).unwrap());
    
    let ptm = PageTableManager::from_pml4(core::mem::transmute(pml4))
        .expect("Failed to crate Page Table Manager");

    //  ptm.register();

    ptm.make_global();

    if let Some(kernel_addr) = KERNEL_ADDRESS_REQUEST.get_response().get() {
        debug!(
            "KERNEL BASE: ( virtual: 0x",
            VirtualAddress::new(kernel_addr.virtual_base as usize).as_str(),
            ", physical: 0x",
            PhysicalAddress::new(kernel_addr.physical_base as usize).as_str(),
            " )",
            endl!()
        );

        match pt_manager!()
            .map_memory(VirtualAddress::new(kernel_addr.virtual_base as usize),
                 PhysicalAddress::new(kernel_addr.physical_base as usize)
            )
        {
            Ok(_) => {},
            Err(paging::table_manager::MemoryMapError::FrameAllocator(e)) => debug_err!("Frame Allocator Error", endl!()),
            Err(paging::table_manager::MemoryMapError::TableNotFound) => debug_err!("One or more tables weren't found for virtual address 0x", VirtualAddress::new(kernel_addr.virtual_base as usize).as_str(), "!", endl!()),
            Err(_) => debug_err!("Other Error", endl!())
        };
    }



    if let Some(kernel_file) = KERNEL_FILE_REQUEST.get_response().get() {
        if let Some(file) = kernel_file.kernel_file.get() {
            assign_uninit! {
                KERNEL_CMDLINE (&'static str) <= core::mem::transmute(file.cmdline.to_str().expect("Failed to get kernel cmdline.").to_str().unwrap())
            }

            assign_uninit! {
                KERNEL_PATH (&'static str) <= core::mem::transmute(file.path.to_str().expect("Failed to get kernel path.").to_str().unwrap())
            }
        }

        debug!("CMDLINE: ", KERNEL_CMDLINE.assume_init(), endl!());
        debug!("PATH: ", KERNEL_PATH.assume_init(), endl!());
    }

    if let Some(module_response) = MODULE_REQUEST.get_response().get() {
        debug!(
            "MODULE COUNT: ",
            integer_to_string(module_response.module_count),
            endl!()
        );
        for module in module_response.modules() {
            kprint!(
                "Found Module ",
                module
                    .path
                    .to_str()
                    .expect("Failed to get Module Path")
                    .to_str()
                    .unwrap(),
                "!"
            );

            let addr =  module.base.as_ptr().unwrap() as usize;


            pt_manager!().map_memory_internal(VirtualAddress::new(addr), PhysicalAddress::new(addr));

            match pt_manager!().get_page_entry(VirtualAddress::new(addr)){
                Ok(_) => {},
                Err(paging::table_manager::MemoryMapError::FrameAllocator(e)) => debug_err!("Frame Allocator Error", endl!()),
                Err(paging::table_manager::MemoryMapError::TableNotFound) => debug_err!("One or more tables weren't found for virtual address 0x", VirtualAddress::new(addr).as_str(), "!", endl!()),
                Err(_) => debug_err!("Other Error", endl!())
            };

            KERNEL_CONSOLE.line_padding = 4;

            kprint!(
                "Module Base: 0x",
                VirtualAddress::new(addr).as_str()
            );
        }
    }

    DEBUG_LINE.unsafe_write_line("End of Runtime.");


    hcf();
}

pub fn boolean_to_str(value: bool) -> &'static str{
    match value{
        true => "true",
        false => "false"
    }
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
