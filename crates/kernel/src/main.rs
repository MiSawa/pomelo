#![no_main]
#![no_std]
#![feature(never_type)]
#![feature(asm_sym)]
#![feature(maybe_uninit_uninit_array)]

use core::{arch::asm, mem::MaybeUninit};
use pomelo_common::BootInfo;

use pomelo_kernel::{
    events, gdt,
    graphics::{canvas::Canvas, console, screen, Color, Rectangle, Size, DESKTOP_BG_COLOR},
    interrupts::{self, InterruptIndex},
    logger, mouse,
    msi::{configure_msi_fixed_destination, DeliveryMode, TriggerMode},
    pci,
    prelude::*,
    xhci,
};

#[no_mangle]
pub extern "sysv64" fn kernel_main(boot_info: &BootInfo) {
    // Just to make sure this function has the expected type signature.
    let _: pomelo_common::KernelMain = kernel_main;

    const KERNEL_MAIN_STACK_SIZE: usize = 1024 * 1024;
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

fn initialize(boot_info: &BootInfo) -> Result<()> {
    screen::initialize(boot_info.graphic_config());
    logger::initialize(log::LevelFilter::Warn)?;
    write_desktop();
    gdt::initialize();
    interrupts::initialize();
    console::initialize(boot_info.graphic_config());
    mouse::initialize(boot_info.graphic_config());
    Ok(())
}

fn write_desktop() {
    let mut screen = screen::screen();
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
}

fn main(boot_info: &BootInfo) -> Result<!> {
    initialize(boot_info)?;
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
    events::event_loop()
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
