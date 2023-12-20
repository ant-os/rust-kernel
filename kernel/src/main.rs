#![recursion_limit = "225"]
#![cfg_attr(not(test), no_std)]
#![no_main]
#![allow(deprecated, incomplete_features, internal_features)]
#![feature(
    panic_info_message,
    unboxed_closures,
    core_intrinsics,
    decl_macro,
    ptr_from_ref,
    inherent_associated_types,
    adt_const_params,
    abi_x86_interrupt,
    allocator_api,
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
    lazy_cell,
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
pub mod debug;
pub mod paging;
pub mod renderer;
pub mod serial;
pub mod status;
pub mod tty;
use alloc::format;
use alloc_impl as _;
use paging::frame_allocator::PageFrameAllocator;
use spin::Mutex;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::paging::mapper::FlagUpdateError;
use x86_64::structures::paging::page::PageRange;
use x86_64::structures::paging::{Mapper, PageTableFlags, PhysFrame, Size2MiB, Size4KiB, Page};
use x86_64::{structures, PhysAddr, VirtAddr};

use crate::alloc_impl::KERNEL_ALLOCATOR;
use crate::common::driver::Driver;
use crate::common::idt::{Idt, KERNEL_IDT};
use crate::common::io::outb;
use crate::common::*;
use crate::device::{
    character::{TimedCharacterDevice, *},
    Device, GeneralDevice,
};
use crate::memory::{
    active_level_4_table, PhysicalAddress, VirtualAddress, PHYSICAL_BOOTLOADER_MEMORY_OFFSET,
    PHYSICAL_MEMORY_OFFSET,
};
use crate::paging::table_manager::PageTableManager;
use core::arch::asm;
use core::ffi::CStr;
use core::hint::unreachable_unchecked;
use elf::endian::AnyEndian;
use elf::segment::ProgramHeader;
use memory::MemoryArea;
pub(crate) use tty::KERNEL_CONSOLE;
#[macro_use]
use core::intrinsics::{likely, unlikely};
#[macro_use]
use core::fmt::*;
use crate::paging::{pf_allocator, pt_manager, PageTable};
use core::mem;
use core::num::NonZeroUsize;
use core::ptr::NonNull;
#[macro_use]
extern crate bitfield;
use numtoa::NumToA;
use renderer::Renderer;

#[macro_use] extern crate antos_macros;

static FRAMEBUFFERS_REQUEST: limine::FramebufferRequest = limine::FramebufferRequest::new(0);
static TERMINAL_REQUEST: limine::TerminalRequest = limine::TerminalRequest::new(0);
static MEMMAP_REQUEST: limine::MemmapRequest = limine::MemmapRequest::new(0);
static KERNEL_ADDRESS_REQUEST: limine::KernelAddressRequest = limine::KernelAddressRequest::new(0);
static KERNEL_FILE_REQUEST: limine::KernelFileRequest = limine::KernelFileRequest::new(0);
static MODULE_REQUEST: limine::ModuleRequest = limine::ModuleRequest::new(0);

static mut SYSTEM_IDT: structures::idt::InterruptDescriptorTable =
    structures::idt::InterruptDescriptorTable::new();

pub(crate) static mut GENERIC_STATIC_BUFFER: [u8; 25] = [0u8; 25];

static TEST1: &'static str = "Hello Paging!";

static TEST2: &'static str = "):";

decl_uninit! {
    ITOA_BUFFER => itoa::Buffer
}

lazy_static::lazy_static! {
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

type DbgPrintFn = unsafe extern "C" fn(*const u8, usize);

extern "x86-interrupt" fn handle_pit(_frame: InterruptStackFrame) {
    unsafe { kdebug!("Tick Tock") }
}

#[repr(C)]
pub struct KernelProgramMeta {
    _dbg_print: *const DbgPrintFn,
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



#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RegisterCapture {
    pub rax: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rbp: u64,
    #[deprecated(note = "This register is used by LLVM.")]
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}


pub static mut INTERRUPT_HANDLERS: [Option<unsafe fn(InterruptStackFrame, RegisterCapture)>; 255] = [None; 255];

static mut RAX: u64 = 0;
static mut RBX: u64 = 0;
static mut RCX: u64 = 0;
static mut RDX: u64 = 0;
static mut RSI: u64 = 0;
static mut RDI: u64 = 0;
static mut RBP: u64 = 0;
static mut RSP: u64 = 0;
static mut R8:  u64 = 0;
static mut R9:  u64 = 0;
static mut R10: u64 = 0;
static mut R11: u64 = 0;
static mut R12: u64 = 0;
static mut R13: u64 = 0;
static mut R14: u64 = 0;
static mut R15: u64 = 0;

/// Captures the Registers of the CPU.
/// 
/// ## Safety
/// 
/// This macro is unsafe because it uses inline assembly.
/// 
/// ## Returns
/// 
/// The Captured Registers.
/// 
/// ## Usage
/// 
/// ```rust
/// let registers = unsafe { capture_registers!() };
/// ```
/// 
/// ## Related
/// 
/// See [`RegisterCapture`] for more info.
/// 
/// [`RegisterCapture`]: kernel::RegisterCapture
pub macro capture_registers() {
    {
        ::core::arch::asm!("#CAPTURE_REGISTERS", out("rax") RAX, out("rcx") RCX, out("rdx") RDX, out("rsi") RSI, out("rdi") RDI, out("r8") R8, out("r9") R9, out("r10") R10, out("r11") R11, out("r12") R12, out("r13") R13, out("r14") R14, out("r15") R15, options(nostack, nomem, preserves_flags));

        RegisterCapture {
            rax: RAX,
            rbx: RBX,
            rcx: RCX,
            rdx: RDX,
            rsi: RSI,
            rdi: RDI,
            rbp: RBP,
            rsp: RSP,
            r8: R8,
            r9: R9,
            r10: R10,
            r11: R11,
            r12: R12,
            r13: R13,
            r14: R14,
            r15: R15,
        }
    }
}

///  Apply Registers and Return from Interrupt.
/// ===========================================
/// 
/// ## Arguments
/// 
/// * `registers` - The Registers to apply.
/// * `capture` - The Capture to apply the Registers from.
/// * `frame` - The Interrupt Stack Frame.
/// 
/// ## Safety
/// This macro is unsafe because it uses inline assembly.
/// 
/// See [`InterruptStackFrame::iretq`] for more info.
/// 
/// See [`__capture_set_registers`] for more info.
/// 
pub macro kernelcall_ret([$($reg:ident)*], $capture:expr, $frame:expr) {
    ::antos_macros::__capture_set_registers!(($($reg),*), $capture);
    $frame.iretq(); // Return from Interrupt.
}

/// The Binding for the [`print`] Kernelcall.
/// 
/// # Arguments
/// 
/// * `buffer` - The Buffer to print.
/// * `len` - The Length of the Buffer.
/// 
/// # Safety
/// 
/// This function is unsafe because it uses inline assembly.
/// 
/// # Returns
/// 
/// The Status of the Kernelcall.
/// 
/// # Usage
/// 
/// To use this function, you have to call it with inline assembly or use AntOS's Kernel Bindings.
/// 
/// ## Example
/// 
/// ```asm
/// mov r9, A_POINTER_TO_THE_STRING
/// mov r10, THE_LENGTH_OF_THE_STRING
/// mov r8, 0x6
/// int 0x80
/// ```
/// 
/// This will call the [`print`] Kernelcall.
/// 
/// [`print`]: kernelcall.print.html
#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn __kernelcall_print(buffer: *const u8, len: u64) -> u64 {
    let mut status: u64;
    asm!(
        "mov r8, 0x6",
        "int 0x80",
        in("r9") buffer,
        in("r10") len,
        lateout("r8") status,
    );
    status
}

pub unsafe fn example_interrupt_handler(_frame: InterruptStackFrame, _capture: RegisterCapture) {
    let mut response = _capture;
    response.rdx = 0x1337;
    
    kernelcall_ret!([rdx], response, _frame);
}

/// Kernelcall for printing to the Screen.
/// This is the Kernel's Implementation of the [`print`] Kernelcall.
/// 
/// # Safety
/// 
/// This function is unsafe, but anything that could go wrong is handled by the Kernel.
/// 
/// # Related
/// See [`print`] for more info.
/// 
/// [`print`]: kernelcall.print.html
pub unsafe fn kernelcall_print(_frame: InterruptStackFrame, _capture: RegisterCapture) {
    let mut response = _capture;
    let string_ptr = response.r9 as *const u8;
    let string_len = response.r10 as usize;

    let string = core::slice::from_raw_parts(string_ptr, string_len);

    let string = core::str::from_utf8_unchecked(string);

    kprint!("{}", string);

    response.r8 = 0x0; // Return 0x0 ( Success ).

    kernelcall_ret!([r8], response, _frame);
}



#[no_mangle]
pub extern "x86-interrupt" fn __irq_handler(_frame: InterruptStackFrame) {
    let mut capture = unsafe { capture_registers!() };
    
    // The Index of the Interrupt Handler is stored in the AL Register.
    let handler_index = (capture.r8) as u8;

    if handler_index == 0 {
        return;
    }

    if let Some(handler) = unsafe { INTERRUPT_HANDLERS[handler_index as usize] } {
        unsafe { handler(_frame, capture) }; // Call the Interrupt Handler, it'll return from the Interrupt.
        unreachable!("Interrupt Handler returned a value!") // The Interrupt Handler should never return.
    } else {
        panic!(
            "Interrupt Handler for Index {} is not defined!",
            handler_index
        );
    }
}

static mut has_panicked: bool = false;

/// The Interrupt Handler for all Exceptions.
/// This function is called when an Exception occurs.
/// 
/// # Arguments
/// 
/// * `stack_frame` - The Interrupt Stack Frame.
/// * `exception_number` - The Exception Number.
/// * `error_code` - The Error Code.
/// 
/// # Usage 
/// This function is called by the **CPU**. It's not meant to be called manually.
#[no_mangle]
#[inline(never)]
pub fn __generic_error_irq_handler(
    stack_frame: InterruptStackFrame,
    exception_number: u8,
    error_code: Option<u64>,
) {
    let exception = unsafe { exception_number.transmute() };
    match exception{
        _ => {
            unsafe {
                if !has_panicked {
                    has_panicked = true;
                    panic!(
                        "Exception: {:?}\nError Code: {:?}\nStack Frame: {:#?}",
                        exception, error_code, stack_frame
                    );
                }
            }
        }
    }
}

pub fn kernel_memmap(virt_addr: VirtAddr, phys_addr: PhysAddr, size: usize, flags: PageTableFlags) {
    use x86_64::structures::paging::PageTableFlags as Flags;
    
    let allocator = unsafe { &mut *KERNEL_FRAME_ALLOCATOR.lock() };

    let page = Page::<Size4KiB>::containing_address(virt_addr);

    let frame = PhysFrame::<Size4KiB>::containing_address(phys_addr);
    let flags = flags;

    let map_to_result = unsafe {
        // FIXME: this is not safe, we do it only for testing
        KERNEL_PAGE_MAPPER.lock().map_to(page, frame, flags, allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

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

    SYSTEM_IDT[0x80 as usize]
        .set_handler_fn(__irq_handler)
        .set_present(true)
        .set_privilege_level(x86_64::PrivilegeLevel::Ring0);

    SYSTEM_IDT[0x20]
        .set_handler_fn(handle_pit)
        .set_present(true);

    x86_64::set_general_handler!(&mut SYSTEM_IDT, __generic_error_irq_handler, 0..28);

    SYSTEM_IDT.load();

    // outb(0xF0, io::inb(0xF0) | 0x100);

    legacy_pic::PRIMARY_PIC.enable(legacy_pic::Interrupt::PIT);

    legacy_pic::PRIMARY_PIC.sync();

    kprint!("Successfully loaded Interrupt Descriptor Table.");

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

                    

                    for page_index in (program_base as u64 / HugePage::SIZE)..((program_base as u64 + module.length as u64) / HugePage::SIZE){
                        let page = HugePage::containing_address(VirtAddr::new((page_index as u64).saturating_mul(HugePage::SIZE)));


            
                        match KERNEL_PAGE_MAPPER.lock().update_flags(
                            page,
                            PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE | PageTableFlags::BIT_55,
                        )
                        {
                            Ok(flusher) => flusher.flush(),
                            Err(FlagUpdateError::PageNotMapped) => {
                                KERNEL_PAGE_MAPPER.lock().identity_map(
                                    PhysFrame::<Size2MiB>::containing_address(PhysAddr::new(page_index as u64 * HugePage::SIZE)),
                                    PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE | PageTableFlags::BIT_55,
                                    &mut *KERNEL_FRAME_ALLOCATOR.lock(),
                                ).unwrap().flush()
                            }
                            Err(e) => {
                                log::error!("Error Updating Page Flags: {:#?}", e);
                                break 'module;
                            }
                        }
                    }

                    log::debug!("Loaded Driver.");

                    let driver = Driver::from_raw_elf(program_base as u64, &bytes, "boot.driver.unknown");

                 //    log::debug!("{}", driver.init());
                }
            }

            //  pt_manager!().map_memory_internal(VirtualAddress::new(addr), PhysicalAddress::new(addr));
        }
    }

    //  kprint!(alloc_impl::format!("{:#?}", pf_allocator!().request_memory_area(((16 * consts::PAGE_SIZE) + 1200) as usize)).as_str());

    //let mut mapper = x86_64::structures::paging::RecursivePageTable::new(pml4_table)
    //    .expect("Failed to create Recursive Page Table Mapper.");

    INTERRUPT_HANDLERS[0x1] = Some(example_interrupt_handler);
    INTERRUPT_HANDLERS[0x6] = Some(kernelcall_print);

    asm!(
        "mov r8, 0x1",
        options(raw, preserves_flags)
    );
    x86_64::software_interrupt!(0x80);
    let mut result: u16;
    asm!("nop", out("rdx") result);

    __kernelcall_print(
        "Hello from the Kernel!\n\0".as_ptr(),
        "Hello from the Kernel!\n\0".len() as u64,
    );

    for (i, e) in KERNEL_PAGE_MAPPER.lock().level_4_table().iter().enumerate() {
        if !e.is_unused() {
            kdebug!(
                "Entry {}: {:#?}\n",
                i,
                e
            );
        }

    }

    log::info!("Hello from the AntOS Kernel! ;)");

    // Breakpoint for GDB.
   // x86_64::instructions::interrupts::int3();

    // log::debug!("MemArea: {:#?}", pf_allocator!().request_memory_area(2000));

    DEBUG_LINE.unsafe_write_line("End of Runtime.");

    loop {
        // GDB_PROTOCOL.lock().update();
        asm!("hlt");
    }
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

    unsafe {
        kdebug!("A Rust Panic has occurred: \n{:#?}\n", _info);
        
        log::error!("A Rust Panic has occurred: \n{:#?}\n", _info.clone());
        
        hcf();
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
