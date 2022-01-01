#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PixelFormat {
    Rgb,
    Bgr,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(&self) -> usize {
        4
    }
    pub fn r_offset(&self) -> u8 {
        match self {
            PixelFormat::Rgb => 0,
            PixelFormat::Bgr => 2,
        }
    }
    pub fn g_offset(&self) -> u8 {
        match self {
            PixelFormat::Rgb => 1,
            PixelFormat::Bgr => 1,
        }
    }
    pub fn b_offset(&self) -> u8 {
        match self {
            PixelFormat::Rgb => 2,
            PixelFormat::Bgr => 0,
        }
    }
}

#[repr(C)]
pub struct GraphicConfig {
    pub frame_buffer_base: *mut u8,
    pub frame_buffer_size: usize,
    pub pixel_format: PixelFormat,
    pub horisontal_resolution: usize,
    pub vertical_resolution: usize,
    pub stride: usize,
}
