#![no_main]
#![no_std]
#![feature(once_cell)]

use core::{arch::asm, format_args};
use pomelo_common::KernelArg;
use pomelo_kernel::{
    canvas::{Canvas, Color, Coordinate, Point},
    screen,
};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    // SCREEN.write().replace(Screen::from(&arg.graphic_config));
    screen::initialize(&arg.graphic_config);
    let screen = screen::screen();
    let mut screen = screen.lock();
    for y in 0..screen.height() {
        for x in 0..screen.width() {
            screen.draw_pixel(Point::new(x, y), Color::WHITE).ok();
        }
    }
    for x in 0..200 {
        for y in 0..100 {
            screen.draw_pixel(Point::new(x, y), Color::GREEN).ok();
        }
    }
    for (i, c) in ('!'..='~').enumerate() {
        screen
            .draw_char(Point::new((i * 8) as Coordinate, 50), Color::BLACK, c)
            .ok();
    }
    screen
        .draw_string(Point::new(0, 66), Color::BLUE, "Hello, world!")
        .ok();
    screen
        .draw_fmt(
            Point::new(0, 82),
            Color::BLACK,
            &mut [0; 32],
            format_args!("1 + 2 = {}", 1 + 2),
        )
        .ok();
    loop {
        unsafe { asm!("hlt") }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[allow(clippy::empty_loop)]
    loop {
        unsafe { asm!("hlt") }
    }
}
