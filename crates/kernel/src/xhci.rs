use mikanos_usb;

use crate::{mouse, pci};

/// Assumes the given func is a xHC, or panic.
pub fn initialize(func: &pci::PCIFunction) -> &mut mikanos_usb::xhci::Controller {
    assert!(matches!(
        func.class(),
        pci::PCIClass::SerialBusController(pci::SerialBusSubclass::USBController(
            pci::USBProgramInterface::XHCI
        ))
    ));
    let bars = func.bars();
    let bar = bars[0] as u64;
    let mmio_base = if bar & 4 == 0 {
        bar
    } else {
        let upper = bars[1] as u64;
        (upper << 32) | bar
    };
    let mmio_base = mmio_base & !0xF;

    let xhc = unsafe { mikanos_usb::xhci::Controller::new(mmio_base) };
    xhc.init();
    xhc.run().map_err(|_| "Failed to initialize xhc").unwrap();
    mikanos_usb::HidMouseDriver::set_default_observer(mouse::observe_cursor_move);
    xhc.configure_connected_ports();
    xhc
}
