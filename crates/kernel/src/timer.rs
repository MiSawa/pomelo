use crate::interrupts::InterruptIndex;

const MAX_TIMER_COUNT: u32 = u32::MAX;

const LVT_TIMER_ADDRESS: *mut u32 = 0xFEE00320 as *mut u32;
const DIVIDE_CONFIGURATION_ADDRESS: *mut u32 = 0xFEE003E0 as *mut u32;
const INITIAL_COUNT_ADDRESS: *mut u32 = 0xFEE00380 as *mut u32;
// const CURRENT_COUNT_ADDRESS: *const u32 = 0xFEE00390 as *const u32;

pub fn initialize() {
    const DIVIDE_1_1: u32 = 0b1011;
    const INTERRUPT: u32 = 1 << 16;
    const PERIODIC: u32 = 1 << 17;
    const VECTOR: u32 = InterruptIndex::LAPICTimer as u32;
    unsafe {
        core::ptr::write(DIVIDE_CONFIGURATION_ADDRESS, DIVIDE_1_1);
        core::ptr::write(LVT_TIMER_ADDRESS, PERIODIC | INTERRUPT | VECTOR);
        core::ptr::write(INITIAL_COUNT_ADDRESS, MAX_TIMER_COUNT);
    }
}
