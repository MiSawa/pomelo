use pomelo_common::memory_mapping::{MemoryDescriptor, MemoryMapping, MemoryType};

const UEFI_PAGE_SIZE: usize = 4096;

pub fn initialize(memory_mapping: &MemoryMapping) {
    for descriptor in memory_mapping.iter() {}
}

fn is_available_type(memory_type: MemoryType) -> bool {
    matches!(
        memory_type,
        MemoryType::BOOT_SERVICES_CODE | MemoryType::BOOT_SERVICES_DATA | MemoryType::CONVENTIONAL
    )
}
