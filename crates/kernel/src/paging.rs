use core::mem::MaybeUninit;

use x86_64::{
    registers::control::{Cr3, Cr3Flags},
    structures::paging::{
        frame::PhysFrame, PageSize, PageTable, PageTableFlags, Size1GiB, Size2MiB,
    },
    PhysAddr,
};

pub fn initialize() {
    // Ah wait this seems to be wrong, I feel like I should obtain the page directory from
    // Cr3::read().
    /// 1GB per page directory
    const PAGE_DIRECTORY_COUNT: usize = 64;
    static mut PML4_TABLE: PageTable = PageTable::new();
    static mut PDP_TABLE: PageTable = PageTable::new();
    static mut PAGE_DIRECTORY: [MaybeUninit<PageTable>; PAGE_DIRECTORY_COUNT] =
        MaybeUninit::uninit_array();

    fn page_table_to_frame(page_table: &PageTable) -> PhysFrame {
        let address = PhysAddr::new(page_table as *const PageTable as u64);
        PhysFrame::from_start_address(address).expect("Page table should be aligned")
    }
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    unsafe {
        PML4_TABLE[0].set_frame(page_table_to_frame(&PDP_TABLE), flags);
    }
    for (i_pdpt, uninitialized_page_table) in unsafe { &mut PAGE_DIRECTORY }.iter_mut().enumerate()
    {
        let initialized = uninitialized_page_table.write(PageTable::new());
        unsafe {
            PDP_TABLE[i_pdpt].set_frame(page_table_to_frame(initialized), flags);
        }
        for (i_pd, directory) in initialized.iter_mut().enumerate() {
            let address =
                PhysAddr::new((i_pdpt as u64) * Size1GiB::SIZE + (i_pd as u64) * Size2MiB::SIZE);
            directory.set_addr(address, flags | PageTableFlags::HUGE_PAGE);
        }
    }
    unsafe {
        Cr3::write(page_table_to_frame(&PML4_TABLE), Cr3Flags::empty());
    }
}
