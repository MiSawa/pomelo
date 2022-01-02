use mikanos_usb;
use spinning_top::Spinlock;

use crate::{mouse, pci};

lazy_static! {
    static ref XHC: Spinlock<Option<&'static mut mikanos_usb::xhci::Controller>> =
        Spinlock::new(Option::None);
}

/// Assumes the given func is a xHC, or panic.
pub fn initialize(func: &pci::PCIFunction) {
    XHC.lock().get_or_insert_with(|| {
        assert!(matches!(
            func.class(),
            pci::PCIClass::SerialBusController(pci::SerialBusSubclass::USBController(
                pci::USBProgramInterface::XHCI
            ))
        ));
        let bars = func.bars();
        log::trace!("bars: {:?}", bars);
        let bar = bars[0] as u64;
        let mmio_base = if bar & 4 == 0 {
            bar
        } else {
            let upper = bars[1] as u64;
            (upper << 32) | bar
        };
        let mmio_base = mmio_base & !0xF;
        log::trace!("mmio base: {:016x}", mmio_base);

        const BUFFER_LEN: usize = 4096 * 32;
        static mut BUFFER: [u8; BUFFER_LEN] = [0; BUFFER_LEN];
        unsafe {
            mikanos_usb::set_memory_pool(BUFFER.as_mut_ptr() as u64, BUFFER.len());
        }

        let xhc = unsafe { mikanos_usb::xhci::Controller::new(mmio_base) };
        xhc.init();
        xhc.run().map_err(|_| "Failed to initialize xhc").unwrap();
        mikanos_usb::HidMouseDriver::set_default_observer(mouse::observe_cursor_move);
        xhc.configure_connected_ports();
        xhc
    });
}

pub(crate) fn handle_events() {
    let mut xhc = XHC.lock();
    let xhc = xhc.as_mut().unwrap();
    while xhc.has_event() {
        if let Err(e) = xhc.process_event() {
            log::error!("Something went wrong while processing xhc event: {}", e.0);
        }
    }
    crate::events::fire_redraw();
}
