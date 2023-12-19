use core::sync::atomic::AtomicBool;

use alloc::format;
use log;

use crate::{kprint, tty::KERNEL_CONSOLE, device::character::UnsafeCharacterDevice, renderer::Renderer};

pub struct KernelLogger{
    pub is_enabled: AtomicBool
}

impl log::Log for KernelLogger{
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.is_enabled.load(core::sync::atomic::Ordering::Relaxed)
    }

    fn log(&self, record: &log::Record) {
        
        kprint!(format!("[{} {}:{}] {}: {}", record.level(), record.file().unwrap_or("<null>"), record.line().unwrap_or(0), record.target(), record.args()).as_str());
    }

    fn flush(&self) { /* empty */}
}

pub(super) static mut KERNEL_LOGGER: KernelLogger = KernelLogger { is_enabled: AtomicBool::new(true) };