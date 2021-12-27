#![no_std]

#[repr(C)]
pub struct KernelArg {
    pub frame_buffer_base: *mut u8,
    pub frame_buffer_size: usize,
}

