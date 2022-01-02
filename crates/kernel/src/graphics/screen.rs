use pomelo_common::graphics::GraphicConfig;

use super::{
    buffer::{BufferCanvas, ByteBuffer},
    Size, UCoordinate,
};

pub type Screen = BufferCanvas<FrameBuffer>;

pub fn create_screen(config: &GraphicConfig) -> Screen {
    Screen::new(
        FrameBuffer::new(config),
        config.pixel_format,
        Size::new(
            config.horisontal_resolution as UCoordinate,
            config.vertical_resolution as UCoordinate,
        ),
        config.pixels_per_row,
    )
}

pub struct FrameBuffer {
    inner: &'static mut [u8],
}
impl FrameBuffer {
    fn new(config: &GraphicConfig) -> Self {
        Self {
            inner: unsafe {
                core::slice::from_raw_parts_mut(config.frame_buffer_base, config.frame_buffer_size)
            },
        }
    }
}
impl ByteBuffer for FrameBuffer {
    fn as_slice(&self) -> &[u8] {
        self.inner
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.inner
    }
}
