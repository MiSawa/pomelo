use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use crate::gdt;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.segment_not_present
            .set_handler_fn(segment_not_present_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::XHCI as usize].set_handler_fn(interrupt_handler_xhci);
        idt[InterruptIndex::LAPICTimer as usize].set_handler_fn(interrupt_handler_lapic_timer);
        idt
    };
}

pub fn initialize() {
    IDT.load();
}

#[repr(u8)]
pub enum InterruptIndex {
    XHCI = 0x40,
    LAPICTimer = 0x41,
}

fn end_of_interrupt() {
    unsafe {
        core::ptr::write_volatile(0xFEE000B0 as *mut u32, 0);
    }
}

extern "x86-interrupt" fn interrupt_handler_xhci(_stack_frame: InterruptStackFrame) {
    log::trace!("Handling XHCI interruption");
    crate::events::fire_xhci();
    end_of_interrupt()
}

extern "x86-interrupt" fn interrupt_handler_lapic_timer(_stack_frame: InterruptStackFrame) {
    log::trace!("Handling LAPIC timer interruption");
    let need_context_switch = crate::task::tick_and_check_context_switch();
    crate::events::fire_lapic_timer();
    end_of_interrupt();
    if need_context_switch {
        if let Err(e) = crate::task::try_switch_context() {
            log::error!("Failed to switch context: {:?}", e);
        }
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::error!("EXCEPTION: BREAKPOINT");
    log::error!("{:#?}", stack_frame);
    end_of_interrupt()
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    log::error!("EXCEPTION: PAGE FAULT");
    log::error!("Accessed Address: {:?}", Cr2::read());
    log::error!("Error Code: {:x}", error_code);
    log::error!("{:#?}", stack_frame);

    loop {
        x86_64::instructions::hlt();
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    log::error!("EXCEPTION: GENERAL PROTECTION FAULT");
    log::error!("Error Code: {:x}", error_code);
    log::error!("{:#?}", stack_frame);

    loop {
        x86_64::instructions::hlt();
    }
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    log::error!("EXCEPTION: STACK NOT PRESENT");
    log::error!("Error Code: {:x}", error_code);
    log::error!("{:#?}", stack_frame);

    loop {
        x86_64::instructions::hlt();
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\nError Code: {:x}\n{:#?}",
        error_code, stack_frame
    );
}
