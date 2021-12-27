#![no_main]
#![no_std]

use core::arch::asm;

#[no_mangle]
pub extern "C" fn kernel_main() -> ! {
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
