const MAX_TIMER_COUNT: u32 = u32::MAX;

const LVT_TIMER_ADDRESS: *mut u32 = 0xFEE00320 as *mut u32;
const INITIAL_COUNT_ADDRESS: *mut u32 = 0xFEE00380 as *mut u32;
const CURRENT_COUNT_ADDRESS: *const u32 = 0xFEE00390 as *const u32;
const DIVIDE_CONFIGURATION_ADDRESS: *mut u32 = 0xFEE003E0 as *mut u32;

pub fn initialize_lapic_timer() {
    const DIVIDE_1_1: u32 = 0b1011;
    const ONESHOT: u32 = 0;
    const MASK: u32 = 1 << 16; // Don't interrupt
    const VECTOR: u32 = 5; // Use 5 as the interruption vector. Why?
    unsafe {
        core::ptr::write(DIVIDE_CONFIGURATION_ADDRESS, DIVIDE_1_1);
        core::ptr::write(LVT_TIMER_ADDRESS, ONESHOT | MASK | VECTOR);
    }
}
pub fn start_lapic_timer() {
    initialize_lapic_timer();
    unsafe {
        core::ptr::write(INITIAL_COUNT_ADDRESS, MAX_TIMER_COUNT);
    }
}
pub fn get_elapsed_time() -> u32 {
    MAX_TIMER_COUNT - unsafe { core::ptr::read(CURRENT_COUNT_ADDRESS) }
}
pub fn stop_lapic_timer() {
    unsafe {
        core::ptr::write(INITIAL_COUNT_ADDRESS, 0);
    }
}
