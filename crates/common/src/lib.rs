#![no_std]

pub mod graphics;
pub mod memory_mapping;

pub type KernelMain = extern "sysv64" fn(&BootInfo);

use graphics::GraphicConfig;
use memory_mapping::MemoryMapping;

#[repr(C)]
pub struct BootInfo {
    graphic_config: GraphicConfig,
    memory_mapping: MemoryMapping,
    acpi2_rsdp: Option<*const core::ffi::c_void>,
}

impl BootInfo {
    pub fn new(
        graphic_config: GraphicConfig,
        memory_mapping: MemoryMapping,
        acpi2_rsdp: Option<*const core::ffi::c_void>,
    ) -> Self {
        Self {
            graphic_config,
            memory_mapping,
            acpi2_rsdp,
        }
    }

    pub fn graphic_config(&self) -> &GraphicConfig {
        &self.graphic_config
    }

    pub fn memory_mapping(&self) -> &MemoryMapping {
        &self.memory_mapping
    }

    pub fn acpi2_rsdp(&self) -> Option<*const core::ffi::c_void> {
        self.acpi2_rsdp
    }
}
