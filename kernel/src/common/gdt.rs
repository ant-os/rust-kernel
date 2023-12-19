//! Global Descriptor Table

pub const GDT_NULL: usize = 0;
pub const GDT_KERNEL_CODE: usize = 1;
pub const GDT_KERNEL_DATA: usize = 2;
pub const GDT_USER_CODE32_UNUSED: usize = 3;
pub const GDT_USER_DATA: usize = 4;
pub const GDT_USER_CODE: usize = 5;
pub const GDT_TSS: usize = 6;
pub const GDT_TSS_HIGH: usize = 7;

pub const GDT_A_PRESENT: u8 = 1 << 7;
pub const GDT_A_RING_0: u8 = 0 << 5;
pub const GDT_A_RING_1: u8 = 1 << 5;
pub const GDT_A_RING_2: u8 = 2 << 5;
pub const GDT_A_RING_3: u8 = 3 << 5;
pub const GDT_A_SYSTEM: u8 = 1 << 4;
pub const GDT_A_EXECUTABLE: u8 = 1 << 3;
pub const GDT_A_CONFORMING: u8 = 1 << 2;
pub const GDT_A_PRIVILEGE: u8 = 1 << 1;
pub const GDT_A_DIRTY: u8 = 1;

pub const GDT_A_TSS_AVAIL: u8 = 0x9;
pub const GDT_A_TSS_BUSY: u8 = 0xB;

pub const GDT_F_PAGE_SIZE: u8 = 1 << 7;
pub const GDT_F_PROTECTED_MODE: u8 = 1 << 6;
pub const GDT_F_LONG_MODE: u8 = 1 << 5;

pub(crate) static mut INIT_GDT: [GdtEntry; 3] = [
    // Null
    GdtEntry::new(0, 0, 0, 0),
    // Kernel code
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_EXECUTABLE | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
    // Kernel data
    GdtEntry::new(0, 0, GDT_A_PRESENT | GDT_A_RING_0 | GDT_A_SYSTEM | GDT_A_PRIVILEGE, GDT_F_LONG_MODE),
];



#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct GdtEntry {
    pub limitl: u16,
    pub offsetl: u16,
    pub offsetm: u8,
    pub access: u8,
    pub flags_limith: u8,
    pub offseth: u8
}

impl GdtEntry {
    pub const fn new(offset: u32, limit: u32, access: u8, flags: u8) -> Self {
        GdtEntry {
            limitl: limit as u16,
            offsetl: offset as u16,
            offsetm: (offset >> 16) as u8,
            access,
            flags_limith: flags & 0xF0 | ((limit >> 16) as u8) & 0x0F,
            offseth: (offset >> 24) as u8
        }
    }

    pub fn set_offset(&mut self, offset: u32) {
        self.offsetl = offset as u16;
        self.offsetm = (offset >> 16) as u8;
        self.offseth = (offset >> 24) as u8;
    }

    pub fn set_limit(&mut self, limit: u32) {
        self.limitl = limit as u16;
        self.flags_limith = self.flags_limith & 0xF0 | ((limit >> 16) as u8) & 0x0F;
    }
}
