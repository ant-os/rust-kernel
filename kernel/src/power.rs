//! Power Manager

use core::convert::Infallible;

use crate::{io::{outb, outw}, kdebug};

#[derive(Debug, Copy, Clone)]
/// A Marker-type for the PowerManager.
pub struct PowerManager;

unsafe impl Sync for PowerManager {}
unsafe impl Send for PowerManager {}

impl PowerManager{
    /// Requests a system Reboot.
    pub fn request_reboot(&self) -> Option<!>{
        unsafe { kdebug!("[Power Manager] Starting reboot...") }

        Some(unsafe { crate::reboot_unchecked() });
    }

    /// Requests a system Shutdown.
    pub fn request_shutdown(&self) -> Option<!>{
        // QEMU Shutdown.
        // TODO: Use ACPI!

        unsafe { kdebug!("[Power Manager] Starting shutdown...\r\n") }
        unsafe { kdebug!("[Power Manager] Terminating using Hypervisor(QEMU) Interface...\r\n") }
        unsafe { outw(0x604, 0x2000); } // Source: https://wiki.osdev.org/Shutdown#Emulator-specific_methods

        Some(panic!("Unsupported Platform, supported Platforms: QEMU(^2.0)"))  
    }
}

pub(crate) const KERNEL_POWER: PowerManager = PowerManager{};
