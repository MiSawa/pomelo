// use crate::x86_64;
use bitfield::bitfield;
use derive_getters::Getters;
use x86_64::instructions::port::{PortReadOnly, PortWriteOnly};

const CONFIG_ADDRESS: u16 = 0x0CF8;
const CONFIG_DATA: u16 = 0x0CFC;

fn read_pci_config(address: u32) -> u32 {
    let mut addr = PortWriteOnly::new(CONFIG_ADDRESS);
    let mut data = PortReadOnly::new(CONFIG_DATA);
    x86_64::instructions::interrupts::without_interrupts(|| unsafe {
        addr.write(address);
        data.read()
    })
}
fn write_pci_config(address: u32, value: u32) {
    let mut addr = PortWriteOnly::new(CONFIG_ADDRESS);
    let mut data = PortWriteOnly::new(CONFIG_DATA);
    x86_64::instructions::interrupts::without_interrupts(|| unsafe {
        addr.write(address);
        data.write(value)
    });
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

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct PCICapabilityHeaderData(u32);
    impl Debug;
    u8; pub capability_id, _: 7, 0;
    u8; next_ptr, _: 15, 8;
    u16; pub capability, _: 31, 16;
}
#[derive(Getters, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct PCICapabilityHeader {
    data: PCICapabilityHeaderData,
    capability_address: u8,
}

impl PCIClass {
    pub fn from_code(base: u8, sub: u8, interface: u8) -> Self {
        match base {
            0x0c => Self::SerialBusController(SerialBusSubclass::from_code(sub, interface)),
            _ => Self::Unimplemented(base, sub, interface),
        }
    }
    pub fn to_code(self) -> (u8, u8, u8) {
        match self {
            Self::SerialBusController(a) => {
                let (b, c) = a.to_code();
                (0x03, b, c)
            }
            Self::Unimplemented(a, b, c) => (a, b, c),
        }
    }
}
impl SerialBusSubclass {
    pub fn from_code(sub: u8, interface: u8) -> Self {
        match sub {
            0x03 => Self::USBController(USBProgramInterface::from_code(interface)),
            _ => Self::Unimplemented(sub, interface),
        }
    }
    pub fn to_code(self) -> (u8, u8) {
        match self {
            Self::USBController(a) => (0x03, a.to_code()),
            Self::Unimplemented(a, b) => (a, b),
        }
    }
}
impl USBProgramInterface {
    pub fn from_code(interface: u8) -> Self {
        match interface {
            0x30 => Self::XHCI,
            _ => Self::Unimplemented(interface),
        }
    }
    pub fn to_code(self) -> u8 {
        match self {
            Self::XHCI => 0x30,
            Self::Unimplemented(a) => a,
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

    pub fn read_conf_register(&self, register_address: u8) -> u32 {
        read_pci_config(make_address(
            self.bus,
            self.device,
            self.function,
            register_address,
        ))
    }
    pub fn write_conf_register(&self, register_address: u8, value: u32) {
        log::info!("{:02x} <- {:08x}", register_address, value);
        write_pci_config(
            make_address(self.bus, self.device, self.function, register_address),
            value,
        )
    }
    pub fn read_bars(&self) -> [u32; 6] {
        let mut bars = [0; 6];
        for i in 0..6 {
            bars[i as usize] = self.read_conf_register(0x10 + 4 * i);
        }
        bars
    }

    fn read_capability_header(&self, capability_address: u8) -> Option<PCICapabilityHeader> {
        if capability_address == 0 {
            None
        } else {
            let data = self.read_conf_register(capability_address);
            Some(PCICapabilityHeader {
                data: PCICapabilityHeaderData(data),
                capability_address,
            })
        }
    }

    pub fn capability_headers(&self) -> impl Iterator<Item = PCICapabilityHeader> + '_ {
        let capability_address = self.read_capability_header(self.read_conf_register(0x34) as u8);
        core::iter::successors(capability_address, |prev| {
            self.read_capability_header(prev.data.next_ptr())
        })
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
