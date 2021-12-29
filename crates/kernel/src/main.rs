#![no_main]
#![no_std]
#![feature(once_cell)]

use core::arch::asm;
use pomelo_common::KernelArg;
use pomelo_kernel::graphics::{
    self, canvas::Canvas, console::Console, screen, Color, ICoordinate, Point, Rectangle, Size,
};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    // SCREEN.write().replace(Screen::from(&arg.graphic_config));
    screen::initialize(&arg.graphic_config);
    let mut screen = screen::screen();
    // let mut screen = screen.lock();

    const DESKTOP_BG_COLOR: Color = Color::new(45, 118, 237);
    const DESKTOP_FG_COLOR: Color = Color::BLACK;

    let screen_size = screen.size();
    screen.fill_rectangle(DESKTOP_BG_COLOR, &screen.bounding_box());
    screen.fill_rectangle(
        Color::new(1, 8, 17),
        &Rectangle::new(
            Point::new(0, screen_size.y as ICoordinate - 50),
            Size::new(screen_size.x, 50),
        ),
    );
    screen.fill_rectangle(
        Color::new(80, 80, 80),
        &Rectangle::new(
            Point::new(0, screen_size.y as ICoordinate - 50),
            Size::new(screen_size.x / 5, 50),
        ),
    );
    screen.fill_rectangle(
        Color::new(160, 160, 160),
        &Rectangle::new(
            Point::new(10, screen_size.y as ICoordinate - 40),
            Size::new(30, 30),
        ),
    );

    let mut console = Console::new(screen, DESKTOP_FG_COLOR, DESKTOP_BG_COLOR);
    console.write_string("Welcome to Pomelo OS!");
    // core::fmt::write(&mut console, format_args!("Wellcome to Pomelo OS!\n")).ok();

    let mut screen = screen::screen();
    graphics::mouse::render_mouse_cursor(&mut screen, Point::new(300, 300));
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
