use arrayvec::ArrayString;
use pomelo_common::graphics::PixelFormat;

use crate::graphics::{
    buffer::{self, FixedSizeBufferCanvas, MAX_BYTES_PER_PIXEL},
    canvas::{Canvas, GLYPH_HEIGHT, GLYPH_WIDTH},
    Color, Draw, Point, Size, UCoordinate,
};

pub struct BufferedArrayText<const N: usize>
where
    [(); N * GLYPH_HEIGHT as usize * GLYPH_WIDTH as usize * MAX_BYTES_PER_PIXEL]: Sized,
{
    text: ArrayString<N>,
    buffer: FixedSizeBufferCanvas<
        { N * GLYPH_HEIGHT as usize * GLYPH_WIDTH as usize * MAX_BYTES_PER_PIXEL },
    >,
    color: Color,
}

impl<const N: usize> BufferedArrayText<N>
where
    [(); N * GLYPH_HEIGHT as usize * GLYPH_WIDTH as usize * MAX_BYTES_PER_PIXEL]: Sized,
{
    pub fn new(color: Color, pixel_format: PixelFormat) -> Self {
        Self {
            text: ArrayString::new(),
            buffer: buffer::new_fixed_size_buffer_canvas::<
                { N * GLYPH_HEIGHT as usize * GLYPH_WIDTH as usize * MAX_BYTES_PER_PIXEL },
            >(
                pixel_format,
                Size::new(N as UCoordinate * GLYPH_WIDTH, GLYPH_HEIGHT),
            ),
            color,
        }
    }
}

impl<const N: usize> Draw for BufferedArrayText<N>
where
    [(); N * GLYPH_HEIGHT as usize * GLYPH_WIDTH as usize * MAX_BYTES_PER_PIXEL]: Sized,
{
    fn size(&self) -> Size {
        Size::new(
            N as UCoordinate * GLYPH_WIDTH as UCoordinate,
            GLYPH_HEIGHT as UCoordinate,
        )
    }

    fn draw<C: Canvas>(&self, canvas: &mut C) {
        canvas.draw_string(self.color, Point::new(0, 0), &self.text);
    }
}
