#![no_main]
#![no_std]
#![feature(never_type)]

use pomelo_common::KernelArg;

use pomelo_kernel::{
    gdt,
    graphics::{canvas::Canvas, console, screen, Color, Rectangle, Size, DESKTOP_BG_COLOR},
    interrupts::{self, InterruptIndex},
    logger, mouse,
    msi::{configure_msi_fixed_destination, DeliveryMode, TriggerMode},
    pci,
    prelude::*,
    xhci,
};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    main(arg).expect("What happened???")
}

fn initialize(arg: &KernelArg) -> Result<()> {
    screen::initialize(&arg.graphic_config);
    logger::initialize(log::LevelFilter::Warn)?;
    write_desktop();
    gdt::initialize();
    interrupts::initialize();
    console::initialize(&arg.graphic_config);
    mouse::initialize(&arg.graphic_config);
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

fn main(arg: KernelArg) -> Result<!> {
    initialize(&arg)?;
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

    // It seems to be enabled already but just to make sure...
    x86_64::instructions::interrupts::enable();
    //
    #[allow(clippy::empty_loop)]
    loop {
        x86_64::instructions::hlt();
    }
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
