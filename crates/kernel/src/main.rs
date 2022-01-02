#![no_main]
#![no_std]
#![feature(never_type)]
#![feature(asm_sym)]
#![feature(maybe_uninit_uninit_array)]
#![feature(alloc_error_handler)]

use core::{arch::asm, mem::MaybeUninit};
use pomelo_common::BootInfo;

use pomelo_kernel::{
    allocator, events, gdt,
    gui::{self, GUI},
    interrupts::{self, InterruptIndex},
    logger,
    msi::{configure_msi_fixed_destination, DeliveryMode, TriggerMode},
    paging, pci,
    prelude::*,
    xhci,
};

#[no_mangle]
pub extern "sysv64" fn kernel_main(boot_info: &BootInfo) {
    // Just to make sure this function has the expected type signature.
    let _: pomelo_common::KernelMain = kernel_main;

    const KERNEL_MAIN_STACK_SIZE: usize = 32 * 1024 * 1024;
    #[repr(align(16))]
    struct Aligned([MaybeUninit<u8>; KERNEL_MAIN_STACK_SIZE]);
    static KERNEL_MAIN_STACK: Aligned = Aligned(MaybeUninit::uninit_array());
    let stack_bottom = KERNEL_MAIN_STACK.0.as_ptr_range().end;
    unsafe {
        asm!(
            "mov rsp, {}", // change the stack pointer
            "mov rdi, {}", // store the arg `boot_info`
            "call {}",     // stack_tricked(boot_info)
            in(reg) stack_bottom,
            in(reg) boot_info,
            sym stack_tricked
        );
    }
    loop {
        x86_64::instructions::hlt();
    }
}
#[no_mangle]
pub extern "sysv64" fn stack_tricked(boot_info: &BootInfo) {
    // Just to make sure this function has the expected type signature.
    let _: pomelo_common::KernelMain = stack_tricked;
    main(boot_info).expect("What happened???")
}

fn initialize(boot_info: &BootInfo) -> Result<GUI> {
    paging::initialize();
    allocator::initialize(boot_info.memory_mapping());
    gdt::initialize();
    logger::initialize(log::LevelFilter::Debug)?;
    let mut gui = gui::create_gui(boot_info.graphic_config());
    gui.render();
    interrupts::initialize();
    Ok(gui)
}

fn main(boot_info: &BootInfo) -> Result<!> {
    let gui = initialize(boot_info)?;
    println!("Welcome to Pomelo OS");
    let xhc = pci::scan_devices()
        .flat_map(|device| device.scan_functions())
        .find(|func| {
            matches!(
                func.class(),
                pci::PCIClass::SerialBusController(pci::SerialBusSubclass::USBController(
                    pci::USBProgramInterface::XHCI
                ))
            )
        })
        .expect("No xHCI was found");

    configure_msi_fixed_destination(
        &xhc,
        TriggerMode::Level,
        DeliveryMode::Fixed,
        InterruptIndex::XHCI as u8,
        0,
    )?;
    log::info!("Initialized xhc interruption");

    xhci::initialize(&xhc);
    log::info!("Initialized xhci");
    events::event_loop(gui)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("I'm panicked!!!! {}", info);
    #[allow(clippy::empty_loop)]
    loop {
        x86_64::instructions::hlt()
    }
}

#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error(info: core::alloc::Layout) -> ! {
    println!("Alloc error!! {:?}", info);
    #[allow(clippy::empty_loop)]
    loop {
        x86_64::instructions::hlt()
    }
}
