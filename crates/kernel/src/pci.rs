use crate::x86_64;
use derive_getters::Getters;

const CONFIG_ADDRESS: u16 = 0x0CF8;
const CONFIG_DATA: u16 = 0x0CFC;

// fn write_address(address: u32) {
//     x86_64::io_out32(CONFIG_ADDRESS, address)
// }
// fn write_data(value: u32) {
//     x86_64::io_out32(CONFIG_DATA, value)
// }
// fn read_data() -> u32 {
//     x86_64::io_in32(CONFIG_DATA)
// }
fn read_pci_config(address: u32) -> u32 {
    unsafe {
        x86_64::io_out32(CONFIG_ADDRESS, address);
        x86_64::io_in32(CONFIG_DATA)
    }
}
fn make_address(bus: u8, device: u8, function: u8, register_address: u8) -> u32 {
    assert!(device >> 5 == 0); // 5 bits
    assert!(function >> 3 == 0); // 3 bits
    (1u32 << 31)
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((register_address as u32) & 0xFC)
}

/// Returns Option<(vendor_id, device_id)>
fn read_ids(bus: u8, device: u8, function: u8) -> Option<(u16, u16)> {
    let ret = read_pci_config(make_address(bus, device, function, 0x00));
    let device_id = (ret >> 16) as u16;
    let vendor_id = ret as u16;
    if vendor_id == u16::MAX {
        None
    } else {
        Some((vendor_id, device_id))
    }
}
/// Assumes valid (bus, device, function) combination
fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
    let ret = read_pci_config(make_address(bus, device, function, 0x0C));
    (ret >> 16) as u8
}
/// Assumes valid (bus, device, function) combination
fn is_singleton_type(bus: u8, device: u8, function: u8) -> bool {
    read_header_type(bus, device, function) & 0x80 == 0
}
/// Assumes valid (bus, device, function) combination
fn read_class(bus: u8, device: u8, function: u8) -> PCIClass {
    let ret = read_pci_config(make_address(bus, device, function, 0x08));
    let base = (ret >> 24) as u8;
    let sub = (ret >> 16) as u8;
    let interface = (ret >> 8) as u8;
    PCIClass::from_code(base, sub, interface)
}
/// Assumes valid (bus, device, function) combination
fn read_bars(bus: u8, device: u8, function: u8) -> [u32; 6] {
    let mut bars = [0; 6];
    for i in 0..6 {
        bars[i as usize] = read_pci_config(make_address(bus, device, function, 0x10 + 4 * i));
    }
    bars
}

// TODO: Add more from https://pci-ids.ucw.cz/read/PD
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum PCIClass {
    /// 0x0C
    SerialBusController(SerialBusSubclass),
    /// Other ones
    Unimplemented(u8, u8, u8),
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SerialBusSubclass {
    /// 0x03
    USBController(USBProgramInterface),
    /// Other ones
    Unimplemented(u8, u8),
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum USBProgramInterface {
    /// 0x30
    XHCI,
    /// Other ones
    Unimplemented(u8),
}

impl PCIClass {
    fn from_code(base: u8, sub: u8, interface: u8) -> Self {
        match base {
            0x0c => Self::SerialBusController(SerialBusSubclass::from_code(sub, interface)),
            _ => Self::Unimplemented(base, sub, interface),
        }
    }
}
impl SerialBusSubclass {
    fn from_code(sub: u8, interface: u8) -> Self {
        match sub {
            0x03 => Self::USBController(USBProgramInterface::from_code(interface)),
            _ => Self::Unimplemented(sub, interface),
        }
    }
}
impl USBProgramInterface {
    fn from_code(interface: u8) -> Self {
        match interface {
            0x30 => Self::XHCI,
            _ => Self::Unimplemented(interface),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Getters)]
pub struct PCIDeviceInfo {
    bus: u8,
    device: u8,
    vendor_id: u16,
    device_id: u16,
}

impl PCIDeviceInfo {
    pub fn scan_functions(&self) -> impl Iterator<Item = PCIFunction> {
        let candidates = if is_singleton_type(self.bus, self.device, 0) {
            0..1
        } else {
            0..8
        };
        let bus = self.bus;
        let device = self.device;
        candidates.filter_map(move |function| {
            read_ids(bus, device, function).map(|(vendor_id, device_id)| {
                PCIFunction::build(bus, device, function, vendor_id, device_id)
            })
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Getters)]
pub struct PCIFunction {
    bus: u8,
    device: u8,
    function: u8,
    vendor_id: u16,
    device_id: u16,
    header_type: u8,
    class: PCIClass,
    bars: [u32; 6],
}

impl PCIFunction {
    fn build(bus: u8, device: u8, function: u8, vendor_id: u16, device_id: u16) -> Self {
        let header_type = read_header_type(bus, device, function);
        let class = read_class(bus, device, function);
        let bars = read_bars(bus, device, function);

        Self {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            header_type,
            class,
            bars,
        }
    }
}

pub fn scan_devices() -> impl Iterator<Item = PCIDeviceInfo> {
    (u8::MIN..=u8::MAX).flat_map(|bus| {
        (0..32).flat_map(move |device| {
            read_ids(bus, device, 0).map(|(vendor_id, device_id)| PCIDeviceInfo {
                bus,
                device,
                vendor_id,
                device_id,
            })
        })
    })
}
