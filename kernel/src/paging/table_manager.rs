use core::{char::UNICODE_VERSION, f64::consts::E, ptr::NonNull};

use limine::NonNullPtr;

use crate::{common::_alloc_frame_as_mut_t, debug, endl, assign_uninit, PAGE_TABLE_MANAGER};

use super::{indexer::PageMapIndexer, PageTable, PageTableEntry};
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
        entry.set_addr((new_table.as_ptr() as usize) >> 12);
        entry.set_present(true);
        entry.set_rw(true);
        new_table
    } else {
        core::mem::transmute(
            NonNull::new(core::mem::transmute::<usize, *mut PageTable>(
                entry.addr().data() << 12,
            ))
            .ok_or(MemoryMapError::TableNotFound)?,
        )
    };

    result
}}

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
        core::arch::asm!("mov {0}, cr3", in(reg) self.PML4.as_ptr());
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

        let pdp = _page_table!(self.PML4, indexer.pdp);

        let pd = _page_table!(pdp, indexer.pd);

        let pt = _page_table!(pd, indexer.pt);

        let mut entry = &mut (&mut *pt.as_ptr()).entries[indexer.p];

        entry.set_addr(physical_addr.data() >> 12);

        entry.set_present(true);

        entry.set_rw(true);

        debug!(
            "Mapped Virtual Address 0x",
            virtual_addr.as_str(),
            " to 0x",
            PhysicalAddress::new(physical_addr.data() >> 12).as_str(),
            endl!()
        );

        Ok(())
    }

    /// Internal Function for Mapping Memory.
    pub(crate) unsafe fn get_page_entry(
        &self,
        virtual_addr: VirtualAddress,
    ) -> Result<PageTableEntry, MemoryMapError> {
        let indexer = PageMapIndexer::for_addr(virtual_addr.data());

        let pdp = _page_table!(self.PML4, indexer.pdp);
        let pd = _page_table!(pdp, indexer.pd);
        let pt = _page_table!(pd, indexer.pt);

        let entry = pt.entries[indexer.p];

        Ok(entry.clone())
    }

    pub fn map_memory(
        &self,
        virtual_addr: VirtualAddress,
        physical_addr: PhysicalAddress,
    ) -> Result<(), MemoryMapError>{
        // TODOOO: Checks.

        unsafe { self.map_memory_internal(virtual_addr, physical_addr) }
    }
}
