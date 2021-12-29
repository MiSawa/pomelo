use core::arch::asm;

pub fn hlt() {
    unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) }
}

pub unsafe fn io_out32(address: u16, data: u32) {
    asm!("out dx, eax", in("dx") address, in("eax") data, options(nomem, nostack, preserves_flags))
}

pub unsafe fn io_in32(address: u16) -> u32 {
    let ret;
    asm!("in eax, dx", in("dx") address, out("eax") ret, options(nomem, nostack, preserves_flags));
    ret
}
