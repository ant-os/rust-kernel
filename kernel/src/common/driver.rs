// Purpose: Common driver structures.

use elf::endian::AnyEndian;

pub type DriverStatus = u64;

/// Driver structure
#[derive(Debug)]
#[repr(C)]
pub struct Driver {
    pub name: &'static str,
    pub signature: &'static str,
    pub driver_entry: DriverEntry,
}

/// Driver entry point
pub type DriverEntry = unsafe fn() -> DriverStatus;

impl Driver {
    /// Creates a new driver
    pub const fn new(name: &'static str, signature: &'static str, driver_entry: DriverEntry) -> Self {
        Self {
            name,
            signature,
            driver_entry,
        }
    }

    /// Returns the driver name
    pub const fn get_name(&self) -> &'static str {
        self.name
    }

    /// Returns the driver signature
    pub const fn get_signature(&self) -> &'static str {
        self.signature
    }

    /// Returns the driver entry point
    /// 
    /// # Safety
    /// 
    /// This function is unsafe because it returns a function pointer.
    pub unsafe fn get_driver_entry(&self) -> DriverEntry {
        self.driver_entry
    }

    /// Initializes the driver
    /// 
    /// # Safety
    /// 
    /// This function is unsafe because it calls the driver entry point.
    /// 
    /// # Returns
    /// 
    /// Returns the driver status.
    /// 
    /// # Notes
    /// 
    /// This function is unsafe because it calls the driver entry point. 
    /// 
    /// It is the responsibility of the caller to ensure that the driver entry point is valid.
    pub unsafe fn init(&self) -> DriverStatus {
        (self.driver_entry)()
    }

    /// Creates a new driver from a raw ELF file
    /// 
    /// # Parameters
    /// 
    /// * `base` - The base address of the ELF file
    /// * `elf` - The ELF file
    /// * `identifier` - The driver identifier
    /// 
    /// # Returns
    /// 
    /// Returns a new driver.
    /// 
    /// # Safety
    /// 
    /// This function is unsafe because it transmutes the driver entry point.
    pub unsafe fn from_raw_elf(base: u64, elf: &elf::ElfBytes<'_, AnyEndian>, identifier: &'static str) -> Self {
        let name = identifier;
        let signature = identifier;
        
        let driver_entry = unsafe { core::mem::transmute(base + elf.ehdr.e_entry)};

        Self::new(name, signature, driver_entry)
    }
}