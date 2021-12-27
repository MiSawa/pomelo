#![no_main]
#![no_std]

use core::arch::asm;
use pomelo_common::KernelArg;

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    let frame_buffer =
        unsafe { core::slice::from_raw_parts_mut(arg.frame_buffer_base, arg.frame_buffer_size) };
    for i in 0..arg.frame_buffer_size {
        frame_buffer[i] = (i % 255) as u8;
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
