#![no_main]
#![no_std]
#![feature(once_cell)]

use core::{arch::asm, format_args};
use pomelo_common::KernelArg;
use pomelo_kernel::graphics::{
    canvas::Canvas,
    console::Console,
    screen::{self, screen},
    Color, ICoordinate, Point, Rectangle, Size,
};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    // SCREEN.write().replace(Screen::from(&arg.graphic_config));
    screen::initialize(&arg.graphic_config);
    let screen = screen();
    let mut screen = screen.lock();
    screen
        .fill_rectangle(Color::WHITE, &screen.bounding_box())
        .ok();
    screen
        .fill_rectangle(
            Color::GREEN,
            &Rectangle::new(Point::zero(), Size::new(200, 100)),
        )
        .ok();
    for (i, c) in ('!'..='~').enumerate() {
        screen
            .draw_char(Color::BLACK, Point::new((i * 8) as ICoordinate, 50), c)
            .ok();
    }
    screen
        .draw_string(Color::BLUE, Point::new(0, 66), "Hello, world!")
        .ok();
    screen
        .draw_fmt(
            Color::BLACK,
            Point::new(0, 82),
            &mut [0; 32],
            format_args!("1 + 2 = {}", 1 + 2),
        )
        .ok();

    let mut console = Console::new(screen, Color::BLACK, Color::WHITE);
    for i in 0..27 {
        core::fmt::write(&mut console, format_args!("print {}\n", i)).ok();
    }
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
