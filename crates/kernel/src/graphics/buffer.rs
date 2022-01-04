use alloc::{vec, vec::Vec};
use pomelo_common::graphics::PixelFormat;

use super::{canvas::Canvas, Color, ICoordinate, Point, Rectangle, Size, Vector2d};

pub const MAX_BYTES_PER_PIXEL: usize = 4;

pub type VecBufferCanvas = BufferCanvas<Vec<u8>>;

pub trait ByteBuffer {
    fn as_slice(&self) -> &[u8];
    fn as_mut_slice(&mut self) -> &mut [u8];
}
impl ByteBuffer for Vec<u8> {
    fn as_slice(&self) -> &[u8] {
        &self[..]
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self[..]
    }
}

impl<const N: usize> ByteBuffer for [u8; N] {
    fn as_slice(&self) -> &[u8] {
        &self[..]
    }
    fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self[..]
    }
}

pub struct BufferCanvas<B> {
    buffer: B,
    pixel_format: PixelFormat,
    r_offset: u8,
    g_offset: u8,
    b_offset: u8,
    size: Size,
    bytes_per_row: usize,
    transparent_color: Option<Color>,
}

impl<B> BufferCanvas<B> {
    pub fn new(buffer: B, pixel_format: PixelFormat, size: Size, pixels_per_row: usize) -> Self {
        Self {
            buffer,
            pixel_format,
            r_offset: pixel_format.r_offset(),
            g_offset: pixel_format.g_offset(),
            b_offset: pixel_format.b_offset(),
            size,
            bytes_per_row: pixels_per_row * pixel_format.bytes_per_pixel(),
            transparent_color: None,
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn set_transparent_color(&mut self, transparent_color: Option<Color>) {
        self.transparent_color = transparent_color;
    }
    pub fn transparent_color(&self) -> Option<Color> {
        self.transparent_color
    }
}

impl BufferCanvas<Vec<u8>> {
    pub fn empty(pixel_format: PixelFormat) -> Self {
        Self::new(vec![0; 0], pixel_format, Size::zero(), 0)
    }

    pub fn vec_backed(pixel_format: PixelFormat, size: Size) -> Self {
        let pixels_per_row = size.x as usize;
        let buffer_len = pixel_format.bytes_per_pixel() * (pixels_per_row * size.y as usize);
        Self::new(vec![0; buffer_len], pixel_format, size, pixels_per_row)
    }

    pub fn resize(&mut self, size: Size) {
        let buffer_len = self.pixel_format.bytes_per_pixel() * (size.x as usize * size.y as usize);
        self.buffer.resize(buffer_len, 0);
        self.size = size;
        self.bytes_per_row = size.x as usize * self.pixel_format.bytes_per_pixel();
    }
}

impl<B> BufferCanvas<B> {
    fn offset_of_pixel(&self, p: Point) -> usize {
        self.bytes_per_row * (p.y as usize) + self.pixel_format.bytes_per_pixel() * (p.x as usize)
    }

    fn read_color(&self, buf: &'_ [u8]) -> Color {
        Color {
            r: buf[self.r_offset as usize],
            g: buf[self.g_offset as usize],
            b: buf[self.b_offset as usize],
        }
    }
}

impl<B: ByteBuffer> BufferCanvas<B> {
    pub fn get_color(&self, p: Point) -> Option<Color> {
        let offset = self.offset_of_pixel(p);
        let buf = &self.buffer.as_slice()[offset..(offset + self.pixel_format.bytes_per_pixel())];
        let ret = Some(self.read_color(buf));
        if ret == self.transparent_color {
            None
        } else {
            ret
        }
    }

    fn draw_to(&self, v: Vector2d, dest: &mut BufferCanvas<impl ByteBuffer>, dest_area: Rectangle) {
        assert_eq!(self.pixel_format, dest.pixel_format);
        let target_rectangle = (self.bounding_box() + v)
            .intersection(&dest.bounding_box())
            .intersection(&dest_area);
        if self.transparent_color.is_some() {
            for p in target_rectangle.points() {
                if let Some(c) = self.get_color(p - v) {
                    dest.draw_pixel_unchecked(c, p)
                }
            }
        } else {
            let mut source_s = self.offset_of_pixel(target_rectangle.top_left() - v);
            let mut source_t = self.offset_of_pixel(target_rectangle.top_right() - v);
            let mut dest_s = dest.offset_of_pixel(target_rectangle.top_left());
            let mut dest_t = dest.offset_of_pixel(target_rectangle.top_right());
            for _ in 0..target_rectangle.height() {
                dest.buffer.as_mut_slice()[dest_s..dest_t]
                    .copy_from_slice(&self.buffer.as_slice()[source_s..source_t]);
                source_s += self.bytes_per_row;
                source_t += self.bytes_per_row;
                dest_s += dest.bytes_per_row;
                dest_t += dest.bytes_per_row;
            }
        }
    }
}

impl<B: ByteBuffer> Canvas for BufferCanvas<B> {
    fn size(&self) -> super::Size {
        self.size
    }

    fn draw_pixel_unchecked(&mut self, color: Color, p: Point) {
        let offset = self.offset_of_pixel(p);
        self.buffer.as_mut_slice()[offset + self.r_offset as usize] = color.r;
        self.buffer.as_mut_slice()[offset + self.g_offset as usize] = color.g;
        self.buffer.as_mut_slice()[offset + self.b_offset as usize] = color.b;
    }
    fn draw_pixel(&mut self, color: Color, p: Point) {
        let size = self.size();
        if p.x < 0 || p.x >= (size.x as ICoordinate) || p.y < 0 || p.y >= (size.y as ICoordinate) {
            return;
        }
        let offset = self.offset_of_pixel(p);
        self.buffer.as_mut_slice()[offset + self.r_offset as usize] = color.r;
        self.buffer.as_mut_slice()[offset + self.g_offset as usize] = color.g;
        self.buffer.as_mut_slice()[offset + self.b_offset as usize] = color.b;
    }
    fn draw_buffer(&mut self, v: Vector2d, buffer: &BufferCanvas<impl ByteBuffer>) {
        buffer.draw_to(v, self, self.bounding_box());
    }
    fn draw_buffer_area(
        &mut self,
        v: Vector2d,
        buffer: &BufferCanvas<impl ByteBuffer>,
        dest_area: Rectangle,
    ) {
        buffer.draw_to(v, self, dest_area);
    }
    fn fill_rectangle(&mut self, color: Color, rectangle: Rectangle) {
        let rectangle = rectangle.intersection(&self.bounding_box());
        let mut pattern = [0; MAX_BYTES_PER_PIXEL];
        pattern[self.r_offset as usize] = color.r;
        pattern[self.g_offset as usize] = color.g;
        pattern[self.b_offset as usize] = color.b;

        let mut s = self.offset_of_pixel(rectangle.top_left());
        let mut t = self.offset_of_pixel(rectangle.top_right());
        let step = self.pixel_format.bytes_per_pixel();
        for _ in 0..rectangle.height() {
            for i in (s..t).step_by(step) {
                self.buffer.as_mut_slice()[i..(i + step)].copy_from_slice(&pattern[0..step]);
            }
            s += self.bytes_per_row;
            t += self.bytes_per_row;
        }
    }
}
