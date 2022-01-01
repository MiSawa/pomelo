use linked_list_allocator::LockedHeap;
use pomelo_common::memory_mapping::MemoryMapping;
use x86_64::structures::paging::page::PageSize;

use crate::memory_manager::{self, FrameSize};

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

const MEMORY_LIMIT: usize = 128 * 1024 * 1024;

pub fn initialize(memory_mapping: &MemoryMapping) {
    let mut mm = memory_manager::initialize(memory_mapping);
    let allocated = mm
        .allocate(MEMORY_LIMIT / FrameSize::SIZE as usize)
        .expect("Unable to allocate memory frame......");
    let start_address = allocated.start.start_address().as_u64() as usize;
    let size = allocated.end.start_address().as_u64() as usize - start_address;
    unsafe { ALLOCATOR.lock().init(start_address, size) }
}
