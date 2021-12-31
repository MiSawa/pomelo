use pomelo_common::memory_mapping::{MemoryMapping, MemoryType};
use spin::Mutex;
use x86_64::{
    structures::paging::{
        frame::{PhysFrame, PhysFrameRange},
        page::{PageSize, Size1GiB, Size4KiB},
    },
    PhysAddr,
};

use crate::bitset::BitSet;

type FrameSize = Size4KiB;
const UEFI_PAGE_SIZE: usize = 4096;
const MAX_PHYSICAL_MEMORY_SIZE: usize = 128 * Size1GiB::SIZE as usize;
const NUM_FRAMES: usize = MAX_PHYSICAL_MEMORY_SIZE.div_floor(FrameSize::SIZE as usize);
static MEMORY_MANAGER: Mutex<BitmapMemoryManager> =
    Mutex::new(BitmapMemoryManager::all_allocated());

pub fn initialize(memory_mapping: &MemoryMapping) {
    let mut mm = MEMORY_MANAGER.lock();
    for descriptor in memory_mapping.iter() {
        if is_available_type(descriptor.ty) {
            let start = PhysAddr::new_truncate(descriptor.phys_start);
            let end = PhysAddr::new_truncate(
                start.as_u64() + descriptor.page_count * (UEFI_PAGE_SIZE as u64),
            );
            let range = PhysFrame::<FrameSize>::range(
                PhysFrame::containing_address(start.align_up(FrameSize::SIZE)),
                PhysFrame::containing_address(end),
            );
            mm.free(range);
        }
    }
}

struct BitmapMemoryManager {
    /// 0 -> unavailable, 1 -> available
    bitset: BitSet<NUM_FRAMES>,
}
impl BitmapMemoryManager {
    const fn all_allocated() -> Self {
        Self {
            bitset: BitSet::new(),
        }
    }

    pub fn allocate(&mut self, num_frames: usize) -> Option<PhysFrameRange<FrameSize>> {
        let mut s = 0;
        'search: while s < NUM_FRAMES - num_frames {
            for i in 0..num_frames {
                if !self.bitset.contains(s + i) {
                    s += i + 1;
                    continue 'search;
                }
            }
            self.bitset.remove_range(s..(s + num_frames));
            let start = PhysAddr::new((s as u64) * FrameSize::SIZE);
            let start = PhysFrame::<FrameSize>::from_start_address(start)
                .expect("...what??? I've multiplied it!!!");
            return Some(PhysFrame::range(start, start + num_frames as u64));
        }
        None
    }

    pub fn free(&mut self, range: PhysFrameRange<FrameSize>) {
        let start = (range.start.start_address().as_u64() / FrameSize::SIZE) as usize;
        let end = (range.end.start_address().as_u64() / FrameSize::SIZE) as usize;
        self.bitset.insert_range(start..end);
    }
}

fn is_available_type(memory_type: MemoryType) -> bool {
    matches!(
        memory_type,
        MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA | MemoryType::CONVENTIONAL
    )
}
