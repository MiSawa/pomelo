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
    for x in 100..300 {
        for y in 100..200 {
            screen.write(x, y, &Color::GREEN);
        }
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
