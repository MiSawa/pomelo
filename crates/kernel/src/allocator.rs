use core::alloc::GlobalAlloc;

use linked_list_allocator::LockedHeap;
use pomelo_common::memory_mapping::MemoryMapping;
use x86_64::structures::paging::page::PageSize;

use crate::memory_manager::{self, FrameSize};

#[global_allocator]
static ALLOCATOR: UninterruptedAlloc = UninterruptedAlloc::empty();

const MEMORY_LIMIT: usize = 768 * 1024 * 1024;

pub fn initialize(memory_mapping: &MemoryMapping) {
    let mut mm = memory_manager::initialize(memory_mapping);
    let allocated = mm
        .allocate(MEMORY_LIMIT / FrameSize::SIZE as usize)
        .expect("Unable to allocate memory frame......");
    let start_address = allocated.start.start_address().as_u64() as usize;
    let size = allocated.end.start_address().as_u64() as usize - start_address;
    ALLOCATOR.init(start_address, size);
}

struct UninterruptedAlloc(LockedHeap);
impl UninterruptedAlloc {
    const fn empty() -> Self {
        Self(LockedHeap::empty())
    }
    fn init(&self, start: usize, size: usize) {
        x86_64::instructions::interrupts::without_interrupts(|| unsafe {
            self.0.lock().init(start, size);
        });
    }
}
unsafe impl GlobalAlloc for UninterruptedAlloc {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        x86_64::instructions::interrupts::without_interrupts(|| self.0.alloc(layout))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        x86_64::instructions::interrupts::without_interrupts(|| self.0.dealloc(ptr, layout))
    }
}
