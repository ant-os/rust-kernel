use core::{char::UNICODE_VERSION, f64::consts::E, ptr::NonNull};

use limine::NonNullPtr;
use x86_64::structures::paging::page_table::PageTableEntry;

use crate::{common::_alloc_frame_as_mut_t, debug, endl, assign_uninit, PAGE_TABLE_MANAGER};

use super::{indexer::PageMapIndexer, PageTable};
use crate::memory::{PhysicalAddress, VirtualAddress};

#[derive(Debug)]
pub enum MemoryMapError {
    FrameAllocator(super::frame_allocator::Error),
    InvalidAddress,
    TableNotFound,
}

#[allow(non_snake_case)]
pub struct PageTableManager {
    PML4: NonNullPtr<PageTable>,
}

 /* 
pub(self) macro _page_table($table:expr, $index:expr) {{
    let mut entry = (&mut (&mut *($table).as_ptr()).entries[$index]);

    let result: NonNullPtr<PageTable> = if !entry.present() {
        let new_table = core::mem::transmute::<NonNull<PageTable>, NonNullPtr<PageTable>>(
            NonNull::<PageTable>::new(
                _alloc_frame_as_mut_t::<PageTable>()
                    .map_err(|err| MemoryMapError::FrameAllocator(err))?,
            )
            .ok_or(MemoryMapError::FrameAllocator(
                super::frame_allocator::Error::OutOfMemory,
            ))?,
        );
        entry.set_addr(crate::memory::PHYSICAL_MEMORY_OFFSET - (new_table.as_ptr() as usize));
        entry.set_present(true);
        entry.set_rw(true);
        new_table
    } else {
        core::mem::transmute(
            NonNull::new(core::mem::transmute::<usize, *mut PageTable>(
                entry.addr().to_virtual().data(),
            ))
            .ok_or(MemoryMapError::TableNotFound)?,
        )
    };

    result
}}*/

impl PageTableManager {
    pub fn new() -> Option<Self> {
        Some(PageTableManager {
            PML4: unsafe {
                core::mem::transmute(NonNull::<PageTable>::new(
                    _alloc_frame_as_mut_t::<PageTable>().ok()?,
                )?)
            },
        })
    }

    pub fn from_pml4(pml4: NonNullPtr<PageTable>) -> Option<Self> {
        Some(PageTableManager { PML4: pml4 })
    }

    pub unsafe fn register(&self) -> &Self{
        self.map_memory(VirtualAddress::new(self.PML4.as_ptr().addr()), PhysicalAddress::new(self.PML4.as_ptr().addr()));
        core::arch::asm!("mov cr3, {0}", in(reg) self.PML4.as_ptr().addr());
        self
    } 

    pub fn make_global(self){
        assign_uninit!(
            PAGE_TABLE_MANAGER (PageTableManager) <= self
        )
    }

    /// Internal Function for Mapping Memory.
    pub(crate) unsafe fn map_memory_internal(
        &self,
        virtual_addr: VirtualAddress,
        physical_addr: PhysicalAddress,
    ) -> Result<(), MemoryMapError> {
        let indexer = PageMapIndexer::for_addr(virtual_addr.data());

        debug!(
            "Not Mapped Virtual Address 0x",
            virtual_addr.as_str(),
            " to 0x",
            PhysicalAddress::new(physical_addr.data()).as_str(),
            endl!()
        );

        Ok(())
    }

  
    pub fn map_memory(
        &self,
        virtual_addr: VirtualAddress,
        physical_addr: PhysicalAddress,
    ) -> Result<(), MemoryMapError>{
        unsafe { self.map_memory_internal(virtual_addr, physical_addr) }
    }


}
