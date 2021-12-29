#![no_main]
#![no_std]
#![feature(never_type)]
#![feature(abi_x86_interrupt)]

use log::{error, info, LevelFilter};
use pomelo_common::KernelArg;
use x86_64::structures::idt::InterruptStackFrame;

use pomelo_kernel::{
    graphics::{canvas::Canvas, console, screen, Color, Rectangle, Size, DESKTOP_BG_COLOR},
    logger, mouse, pci,
    prelude::*,
    xhci,
};

#[no_mangle]
pub extern "C" fn kernel_main(arg: KernelArg) -> ! {
    main(arg).expect("What happened???")
}

fn initialize(arg: &KernelArg) -> Result<()> {
    screen::initialize(&arg.graphic_config);
    logger::initialize(LevelFilter::Debug)?;
    write_desktop();
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
    // write_desktop();
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
    let xhc = xhci::initialize(&xhc);
    info!("Initialized xhci");

    loop {
        if let Err(e) = xhc.process_event() {
            error!("Something went wrong: {}", e.0);
        }
    }
}

extern "x86-interrupt" fn interrupt_handler_xhci(_stack_frame: InterruptStackFrame) {
    print!(".");
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    #[allow(clippy::empty_loop)]
    loop {
        x86_64::instructions::hlt()
    }
}
