use alloc::{vec, vec::Vec};
use pomelo_common::graphics::PixelFormat;

use super::{canvas::Canvas, Color, ICoordinate, Point, Rectangle, Size, Vector2d};

const MAX_BYTES_PER_PIXEL: usize = 4;

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

pub struct BufferCanvas<B> {
    buffer: B,
    pixel_format: PixelFormat,
    r_offset: u8,
    g_offset: u8,
    b_offset: u8,
    size: Size,
    stride: usize,
    transparent_color: Option<Color>,
}

impl<B> BufferCanvas<B> {
    pub fn new(buffer: B, pixel_format: PixelFormat, size: Size, stride: usize) -> Self {
        Self {
            buffer,
            pixel_format,
            r_offset: pixel_format.r_offset(),
            g_offset: pixel_format.g_offset(),
            b_offset: pixel_format.b_offset(),
            size,
            stride,
            transparent_color: None,
        }
    }
}

impl BufferCanvas<Vec<u8>> {
    pub fn vec_backed(pixel_format: PixelFormat, size: Size) -> Self {
        let buffer_len = (size.x as usize) * (size.y as usize) * pixel_format.bytes_per_pixel();
        let stride = size.x as usize * pixel_format.bytes_per_pixel();
        Self::new(vec![0; buffer_len], pixel_format, size, stride)
    }
}

impl<B> BufferCanvas<B> {
    fn offset_of_pixel(&self, p: Point) -> usize {
        self.pixel_format.bytes_per_pixel() * (self.stride * (p.y as usize) + (p.x as usize))
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

    fn draw_to(&self, p: Vector2d, dest: &mut BufferCanvas<impl ByteBuffer>) {
        assert_eq!(self.pixel_format, dest.pixel_format);
        let target_rectangle = (self.bounding_box() + p).intersection(&dest.bounding_box());
        let mut source_s = self.offset_of_pixel(target_rectangle.top_left() - p);
        let mut source_t = self.offset_of_pixel(target_rectangle.top_right() - p);
        let mut dest_s = dest.offset_of_pixel(target_rectangle.top_left());
        let mut dest_t = dest.offset_of_pixel(target_rectangle.top_right());
        for _ in 0..target_rectangle.height() {
            dest.buffer.as_mut_slice()[dest_s..dest_t]
                .copy_from_slice(&self.buffer.as_slice()[source_s..source_t]);
            source_s += self.stride;
            source_t += self.stride;
            dest_s += dest.stride;
            dest_t += dest.stride;
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
    fn draw_buffer(&mut self, p: Vector2d, buffer: &BufferCanvas<impl ByteBuffer>) {
        buffer.draw_to(p, self);
    }
    fn fill_rectangle(&mut self, color: Color, rectangle: &Rectangle) {
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
            s += self.stride * step;
            t += self.stride * step;
        }
    }
}
