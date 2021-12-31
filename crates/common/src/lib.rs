#![no_std]

pub mod graphics;
pub mod memory_mapping;

pub type KernelMain = extern "sysv64" fn(BootInfo);

use graphics::GraphicConfig;
use memory_mapping::MemoryMapping;

#[repr(C)]
pub struct BootInfo {
    graphic_config: GraphicConfig,
    memory_mapping: MemoryMapping,
}

impl BootInfo {
    pub fn new(graphic_config: GraphicConfig, memory_mapping: MemoryMapping) -> Self {
        Self {
            graphic_config,
            memory_mapping,
        }
    }

    pub fn graphic_config(&self) -> &GraphicConfig {
        &self.graphic_config
    }

    pub fn memory_mapping(&self) -> &MemoryMapping {
        &self.memory_mapping
    }
}
