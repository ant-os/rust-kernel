#![recursion_limit = "225"]
#![cfg_attr(not(test), no_std)]
#![no_main]
#![allow(deprecated, incomplete_features, internal_features, warnings)]
#![feature(
    panic_info_message,
    unboxed_closures,
    core_intrinsics,
    decl_macro,
    ptr_from_ref,
    inherent_associated_types,
    adt_const_params,
    ascii_char,
    abi_x86_interrupt,
    allocator_api,
    const_for,
    effects,
    const_trait_impl,
    const_mut_refs,
    portable_simd,
    strict_provenance,
    sync_unsafe_cell,
    debug_closure_helpers,
    if_let_guard,
    let_chains,
    panic_internals,
    marker_trait_attr,
    asm_const,
    type_name_of_val,
    alloc_internals,
    str_from_raw_parts,
    lazy_cell
)]

extern crate alloc;

pub mod alloc_impl;
pub mod bitmap_font;
pub mod common;
pub mod device;
pub mod framebuffer;
pub mod graphics;
pub mod kernel_logger;
pub mod legacy_pic;
pub mod memory;
pub mod paging;
pub mod renderer;
pub mod rsdp;
pub mod serial;
pub mod status;
pub mod tty;
pub mod vtree;
use crate::alloc_impl::KERNEL_ALLOCATOR;
use crate::common::idt::{Idt, KERNEL_IDT};
use crate::common::io::outb;
use crate::common::*;
use crate::consts::PAGE_SIZE;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use crate::memory::{
    active_level_4_table, PhysicalAddress, VirtualAddress, PHYSICAL_BOOTLOADER_MEMORY_OFFSET,
    PHYSICAL_MEMORY_OFFSET,
};
use crate::paging::table_manager::PageTableManager;
use crate::rsdp::{Rsdp, RsdpBase};
use alloc::collections::BTreeMap;
use alloc::string::*;
use alloc::{borrow::ToOwned, format, vec::Vec};
use alloc_impl as _;
use common::io::inb;
use core::arch::asm;
use core::ffi::CStr;
use core::hint::unreachable_unchecked;
use core::ops::{Deref, Rem};
use elf::endian::AnyEndian;
use elf::segment::ProgramHeader;
use heapless::sorted_linked_list::LinkedIndexUsize;
use memory::MemoryArea;
use paging::frame_allocator::PageFrameAllocator;
use spin::Mutex;
pub(crate) use tty::KERNEL_CONSOLE;

use x86_64::structures::idt::{InterruptStackFrame, InterruptStackFrameValue};
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, PageTableFlags, PhysFrame, Size2MiB, Size4KiB,
};
use x86_64::{structures, PhysAddr, VirtAddr};
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use crate::paging::{pf_allocator, pt_manager, PageTable};
use core::mem::{self, size_of, transmute};
use core::num::NonZeroUsize;
use core::ptr::{slice_from_raw_parts, NonNull};
#[macro_use]
extern crate bitfield;
use numtoa::NumToA;
use renderer::Renderer;

static FRAMEBUFFERS_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static KERNEL_ADDRESS_REQUEST: limine::KernelAddressRequest = limine::KernelAddressRequest::new(0);
static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(0);
static MODULE_REQUEST: limine::ModuleRequest = limine::ModuleRequest::new(0);
static FDB_REQUEST: limine::DtbRequest = limine::DtbRequest::new(0);
static SMP_REQUEST: limine::SmpRequest = limine::SmpRequest::new(0);
static RSDP_REQUEST: limine::RsdpRequest = limine::RsdpRequest::new(0);

static mut PROGRAM_INLINE: &[u8] = include_bytes!("../../PROGRAM.SYS");

static mut SYSTEM_IDT: structures::idt::InterruptDescriptorTable =
    structures::idt::InterruptDescriptorTable::new();

pub(crate) static mut GENERIC_STATIC_BUFFER: [u8; 25] = [0u8; 25];

static TEST1: &'static str = "Hello Paging!";

static TEST2: &'static str = "):";

decl_uninit! {
    ITOA_BUFFER => itoa::Buffer
}

// Initalized all "pre-entry" variables.
lazy_static::lazy_static! {
    pub(crate) static ref RSDP_VALUE: &'static u8 = {
        if let Some(rsdp_resp) = RSDP_REQUEST.get_response().get(){
            unsafe { rsdp_resp.address.get::<'static>().expect("Invalid RSDP Address")}
        }else{
            panic!("Cannot get RSDP");
        }
    };
    pub(crate) static ref FRAMEBUFFERS: &'static [NonNullPtr<limine::Framebuffer>] = {
        if let Some(fb_resp) = FRAMEBUFFERS_REQUEST.get_response().get(){
            unsafe { core::mem::transmute(fb_resp.framebuffers()) }
        }else{
            debug_err!("Failed to get the list of System Framebuffers!");
            panic!("Failed to get the list of System Framebuffers!");
        }
    };
    pub(crate) static ref KERNEL_BASE: &'static limine::KernelAddressResponse = {
        if let Some(resp) = KERNEL_ADDRESS_REQUEST.get_response().get::<'static>(){
            resp
        }else{
            debug_err!("Failed to get the list of System Framebuffers!");
            log::error!("Failed to get the list of System Framebuffers!");
            panic!("Failed to get the list of System Framebuffers!");
        }
    };
    static ref KERNEL_FRAME_ALLOCATOR: Mutex<PageFrameAllocator> = Mutex::new(unsafe { PageFrameAllocator::from_response(&*KERNEL_MEMMAP) });
    static ref KERNEL_PAGE_TABLE_MANAGER: Mutex<PageTableManager> = Mutex::new(PageTableManager::new().expect("Failed to create Page Table Manager."));
    static ref KERNEL_PAGE_MAPPER: Mutex<x86_64::structures::paging::OffsetPageTable::<'static>> = Mutex::new(unsafe { x86_64::structures::paging::mapper::OffsetPageTable::new(unsafe { active_level_4_table(VirtAddr::zero()) }, VirtAddr::zero())});
    pub(crate) static ref KERNEL_FILE: &'static limine::File = {
        if let Some(resp) = KERNEL_FILE_REQUEST.get_response().get(){
            resp.kernel_file.get::<'static>().unwrap()
        }else{
            debug_err!("Failed to get the list of System Framebuffers!");
            log::error!("Failed to get the list of System Framebuffers!");
            panic!("Failed to get the list of System Framebuffers!");
        }
    };
    #[doc = "The Area of Memory the Kernel Uses."]
    static ref KERNEL_AREA: MemoryArea = MemoryArea::new(KERNEL_BASE.virtual_base as usize, KERNEL_FILE.length as usize);
    static ref L4_PAGE_TABLE: &'static mut PageTable = unsafe { active_level_4_table(VirtAddr::zero()) };
    static ref KERNEL_MEMMAP: &'static limine::MemmapResponse = {
        if let Some(resp) = MEMMAP_REQUEST.get_response().get(){
            resp
        }else{

            debug_err!("Failed to get the list of System Framebuffers!");
            log::error!("Failed to get the list of System Framebuffers!");
            panic!("Failed to get the list of System Framebuffers!");
        }
    };
}

#[derive(Debug, Clone, Copy)]
enum OutlineSegment {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CurveTo(f32, f32, f32, f32, f32, f32),
    Stop,
}

type HugePage = x86_64::structures::paging::Page<Size2MiB>;

#[no_mangle]
unsafe extern "C" fn __prog_debug_print(__base: *const u8, __len: usize) {
    KERNEL_CONSOLE.write_str(core::str::from_utf8_unchecked(core::slice::from_raw_parts(
        __base, __len,
    )));
}

extern "x86-interrupt" fn handle_pit(_frame: InterruptStackFrame) {
    unsafe { kdebug!("Tick Tock") }
}

const MAX_FONT_OUTLINE_SEGMENTS: usize = 25;

pub struct FontOutline(heapless::Vec<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS>);

impl FontOutline {
    pub const fn new() -> Self {
        Self(heapless::Vec::<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS>::new())
    }

    pub const fn segments(&self) -> &'_ heapless::Vec<OutlineSegment, MAX_FONT_OUTLINE_SEGMENTS> {
        &(self.0)
    }

    pub fn push(&mut self, seg: OutlineSegment) {
        self.0.push(seg).expect("Failed to push Font Segment");
    }
}

/// Restart with out checks... (e.g  [unreachable_unchecked])
#[inline]
pub unsafe fn reboot_unchecked() -> ! {
    let mut good: u8 = 0x02;
    while (good & 0x02) != 0 {
        good = inb(0x64);
    }
    outb(0x64, 0xfe);

    unreachable_unchecked();
}

impl ttf_parser::OutlineBuilder for FontOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push(OutlineSegment::MoveTo(x, y))
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.push(OutlineSegment::LineTo(x, y))
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.push(OutlineSegment::QuadTo(x1, y1, x, y))
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.push(OutlineSegment::CurveTo(x1, y1, x2, y2, x, y))
    }

    fn close(&mut self) {
        self.push(OutlineSegment::Stop)
    }
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

pub type InterruptParams = (usize, *mut ());

#[no_mangle]
pub extern "x86-interrupt" fn __irq_handler(_frame: InterruptStackFrame) {
    let mut ptr: *const u8;
    let mut len: usize;

    unsafe {
        asm!(
            "nop", out("r8w") ptr, out("r9w") len
        );
    }

    unsafe {
        kdebug!("kprint invoked.");
    }

    let string = unsafe { core::str::from_raw_parts(ptr, len) };

    unsafe {
        DEBUG_LINE.unsafe_write_string(string);
    }

    unsafe { _frame.iretq() };
}

static mut has_panicked: bool = false;

fn __generic_error_irq_handler(
    stack_frame: InterruptStackFrame,
    index: u8,
    error_code: Option<u64>,
) {
    unsafe {
        // Renderer::global_mut().clear(0xFF0000FF);
        Renderer::global_mut().update_colors(Some(0xFF0000FF), Some(0xFF0000FF));
        KERNEL_CONSOLE.cursor_pos = (1, 1);
        KERNEL_CONSOLE.print("=== PANIC ===");
        log::error!(
            "A Exception has happend: ( error = {:?}, index = {:?}, frame = {:#?}",
            error_code,
            index,
            stack_frame
        );
        kdebug!(
            "A Exception has happend: ( error = {:?}, index = {:?}, frame = {:#?}",
            error_code,
            index,
            stack_frame
        );

        hcf();
    };
}

#[repr(C)]
pub struct DriverObject {}

#[no_mangle]
unsafe extern "C" fn _start<'kernel>() -> ! {
    // let formated = buffer.format(123);

    let PRIMARY_FONT: Option<limine::File> = None;

    let mut kernel_renderer = renderer::Renderer::new(
        FRAMEBUFFERS.get(0).expect("No System Framebuffers."),
        &FONT_BITMAP,
    );

    let color = graphics::Color::from_rgb(0, 255, 0);

    kernel_renderer.update_colors(Some(0xFFFFFFFF), Some(color.inner()));
    kernel_renderer.clear(color.inner()); // 0xFF010FFF
    kernel_renderer.optional_font_scaling = Some(2);

    Renderer::make_global(kernel_renderer);

    kprint!(
        "(c) 2023 Joscha Egloff & AntOS Project. See README.MD for more info.\n",
        "AntOS Kernel ( ",
        git_version::git_version!(),
        " )"
    );

    KERNEL_CONSOLE.newline();

    kprint!("Console Successfully Initialized.\n");

    DEBUG_LINE.wait_for_connection();

    log::set_logger(&kernel_logger::KERNEL_LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    alloc_impl::KERNEL_ALLOCATOR
        .initialize()
        .expect("Failed to initialized Global Allocator.");

    // kprint!(alloc_impl::format!("Remaining Allocator Arena: {}", alloc_impl::KERNEL_ALLOCATOR.get_arena_size()).as_str());

    if let Some(kernel_addr) = KERNEL_ADDRESS_REQUEST.get_response().get() {
        kprint!(
            "KERNEL BASE: ( virtual: 0x",
            VirtualAddress::new(kernel_addr.virtual_base as usize).as_str(),
            ", physical: 0x",
            PhysicalAddress::new(kernel_addr.physical_base as usize).as_str(),
            " )",
            endl!()
        );
    }
    x86_64::set_general_handler!(&mut SYSTEM_IDT, __generic_error_irq_handler, 0..28);

    SYSTEM_IDT[0x80 as usize]
        .set_handler_fn(__irq_handler)
        .set_present(true)
        .set_privilege_level(x86_64::PrivilegeLevel::Ring0);

    SYSTEM_IDT[0x30]
        .set_handler_fn(handle_pit)
        .set_present(true);

    SYSTEM_IDT.load();

    let rsdp: &'static Rsdp = transmute(*RSDP_VALUE);

    kdebug!("RSDP: {:#?}", rsdp.xsdp);

    // outb(0xF0, io::inb(0xF0) | 0x100);

    legacy_pic::PRIMARY_PIC.enable(legacy_pic::Interrupt::PIT);

    legacy_pic::PRIMARY_PIC.sync();

    kprint!("Successfully loaded Interrupt Descriptor Table.");

    if let Some(smp_resp) = SMP_REQUEST.get_response().get() {
        kdebug!("Found SMP: {:#?}", smp_resp);
    }

    if let Some(module_response) = MODULE_REQUEST.get_response().get() {
        kprint!(
            "MODULE COUNT: ",
            integer_to_string(module_response.module_count),
            endl!()
        );
        for module in module_response.modules() {
            let path = module
                .path
                .to_str()
                .expect("Failed to get Module Path")
                .to_str()
                .unwrap();

            let cmdline = module
                .cmdline
                .to_str()
                .expect("Failed to get Module Path")
                .to_str()
                .unwrap();

            let addr = module.base.as_ptr().unwrap() as usize;

            kprint!(
                "$BOOT$",
                path,
                ": Successfully loaded... {\n parameters = [ ",
                cmdline,
                " ],\n base = 0x",
                VirtualAddress::new(addr).as_str(),
                "\n}"
            );

            'module: {
                if path.contains(".TTF") {
                    let face_result = ttf_parser::Face::parse(
                        core::slice::from_raw_parts(
                            module.base.as_ptr().unwrap(),
                            module.length as usize,
                        ),
                        0,
                    );

                    if let Ok(face) = face_result {
                        let id = face.glyph_index('A').unwrap();

                        let mut builder = FontOutline::new();

                        face.outline_glyph(id, &mut builder).unwrap();

                        kprint!(
                            "Font has ",
                            integer_to_string(builder.segments().len()),
                            " Segments!"
                        );
                    }
                } else if path.contains(".SYS") {
                    let program_base = module.base.as_ptr().expect("No Module Base");
                    let program = core::slice::from_raw_parts(program_base, module.length as usize);

                    let bytes = match elf::ElfBytes::<AnyEndian>::minimal_parse(program) {
                        Err(e) => {
                            log::error!("Error Parsing Program: {:#?}", e);
                            break 'module;
                        }
                        Ok(v) => v,
                    };
                    kdebug!("{:x?}\n", program_base.addr());
                    kdebug!("{:x?}\n", program_base.addr() as u64 + bytes.ehdr.e_entry);
                    kdebug!("{:x?}\n", bytes.dynamic_symbol_table());

                    for page in KERNEL_PAGE_MAPPER.lock().level_4_table().iter() {
                        if page.is_unused() {
                            continue;
                        }
                        kdebug!("{:#?}", page);
                    }
                }
            }

            //  pt_manager!().map_memory_internal(VirtualAddress::new(addr), PhysicalAddress::new(addr));
        }
    }

    //  kprint!(alloc_impl::format!("{:#?}", pf_allocator!().request_memory_area(((16 * consts::PAGE_SIZE) + 1200) as usize)).as_str());

    //let mut mapper = x86_64::structures::paging::RecursivePageTable::new(pml4_table)
    //    .expect("Failed to create Recursive Page Table Mapper.");

    log::info!("Hello from the AntOS Kernel! ;)");

    // log::debug!("MemArea: {:#?}", pf_allocator!().request_memory_area(2000));

    let mut cwd = "//?/C:".to_string();

    let mut vtree = vtree::VTree::new();

    vtree
        .nodes
        .insert("?".to_owned(), vtree::Node::Directory(BTreeMap::new()));

    vtree
        .builder("?")
        .unwrap()
        .attach_or_update("C:".to_owned(), vtree::Node::Directory(BTreeMap::new()))
        .attach_or_update(
            "$MODULES$".to_owned(),
            vtree::Node::Directory(BTreeMap::new()),
        );

    kdebug!("TreeRoot: {:?}", vtree.find("//?/").map(|n| n.children()));

    loop {
        kdebug!("{}>", cwd);
        if let Some(line) = DEBUG_LINE.read_line() {
            let segments = line.trim().split(' ').collect::<Vec<_>>();

            if segments.len() == 0 {
                continue;
            }

            match segments[0] {
                "" => continue,
                "ls" => {
                    kdebug!(
                        "{:?}\r\n",
                        vtree
                            .find(cwd.as_str())
                            .expect("not found")
                            .children()
                            .iter()
                            .cloned()
                            .map(|(n, _)| n)
                            .collect::<Vec<String>>()
                            .join("\r\n")
                    );
                }
                "cd" => {
                    let path = segments[1];

                    if path == ".." {
                        cwd = {
                            let mut parts =
                                cwd.trim_end_matches('/').split('/').collect::<Vec<_>>();

                            parts.pop();

                            parts.join("/")
                        };
                    } else {
                        cwd = path.to_string();
                    };
                }
                "clear" => DEBUG_LINE.unsafe_write_string(r"\e[3J\ec"),
                cmd => kdebug!("The command {:?} cannot be found.\r\n", cmd),
            }
        }
    }

    DEBUG_LINE.unsafe_write_line("End of Runtime.");

    // loop {}
}

pub fn boolean_to_str(value: bool) -> &'static str {
    match value {
        true => "true",
        false => "false",
    }
}

macro_rules! halt_while {
    ($boolean_expr:expr) => {
        while ($boolean_expr) {
            asm!("hlt");
        }
    };
}

#[cfg(not(test))]
#[panic_handler]
fn rust_panic(_info: &core::panic::PanicInfo) -> ! {
    use core::{f32::NEG_INFINITY, hint::unreachable_unchecked};

    unsafe {
        kdebug!("The AntOS Kernel Panicked: \r\n{:#?}", _info);
    }

    log::error!("A Rust Panic has occurred: \n{:#?}\n", _info);

    unsafe {
        reboot_unchecked();
    }
}
fn hcf() -> ! {
    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}
