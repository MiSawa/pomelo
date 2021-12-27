#![no_main]
#![no_std]

use core::arch::asm;
use pomelo_common::KernelArg;

mod screen;
use screen::{Color, Screen};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    let mut screen = Screen::from(&arg.graphic_config);
    for x in 0..screen.width() {
        for y in 0..screen.height() {
            screen.write(x, y, &Color::WHITE);
        }
    }
    for x in 0..200 {
        for y in 0..100 {
            screen.write(x, y, &Color::GREEN);
        }
    }
    for (i, c) in (b'!'..=b'~').enumerate() {
        screen.write_char(i * 8, 50, c, &Color::BLACK);
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
