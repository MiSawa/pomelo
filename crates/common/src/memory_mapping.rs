pub use uefi::table::boot::{MemoryDescriptor, MemoryType};

#[repr(C)]
pub struct MemoryMapping {
    pointer: *const MemoryDescriptor,
    len: usize,
}

impl MemoryMapping {
    pub fn new(descriptors: &'static [MemoryDescriptor]) -> Self {
        Self {
            pointer: descriptors.as_ptr(),
            len: descriptors.len(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MemoryDescriptor> {
        // SAFETY: Self can be built only from &'static [MemoryDescriptor]. We just convert it
        // back to that representation.
        let descriptors = unsafe { core::slice::from_raw_parts(self.pointer, self.len) };
        descriptors.iter()
    }
}
