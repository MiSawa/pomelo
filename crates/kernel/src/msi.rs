use bitfield::bitfield;

use crate::{
    pci::{PCICapabilityHeader, PCIFunction},
    prelude::*,
};

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TriggerMode {
    Edge = 0,
    Level = 1,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum DeliveryMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    SMI = 0b010,
    NMI = 0b100,
    INIT = 0b101,
    ExtINT = 0b111,
}

bitfield! {
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MSICapabilityHeaderData(u32);
    impl Debug;
    u8; pub capability_id, _: 7, 0;
    u8; next_ptr, _: 15, 8;
    msi_enable, set_msi_enable: 16;
    u8; multi_message_capable, _: 19, 17;
    u8; multi_message_enable, set_multi_message_enable: 22, 20;
    addr_64_capable, _: 23;
    per_vector_mask_capable, _: 24;
    // u8; _, _: 31, 25;
}
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct MSICapability {
    header: MSICapabilityHeaderData,
    capability_address: u8,
    message_address: u32,
    message_upper_address: u32,
    message_data: u32,
    mask_bits: u32,
    pending_bits: u32,
}

pub fn configure_msi_fixed_destination(
    func: &PCIFunction,
    trigger_mode: TriggerMode,
    delivery_mode: DeliveryMode,
    vector: u8,
    num_vector_exponent: u8,
) -> Result<()> {
    let apic_id = read_local_apic_id();
    let message_address = 0xFEE00000 | ((apic_id as u32) << 12);
    let mut message_data = ((delivery_mode as u32) << 8) | (vector as u32);
    if trigger_mode == TriggerMode::Level {
        message_data |= 0xC000;
    }
    configure_msi(func, message_address, message_data, num_vector_exponent)
}

fn read_msi_capability(
    func: &PCIFunction,
    pci_capability_header: PCICapabilityHeader,
) -> Option<MSICapability> {
    const CAPABILITY_MSI: u8 = 0x05;
    // const CAPABILITY_MSIX: u8 = 0x11;
    if pci_capability_header.data().capability_id() != CAPABILITY_MSI {
        return None;
    }
    // Read MSI capability
    let mut msi_capability = MSICapability {
        header: MSICapabilityHeaderData(0),
        capability_address: *pci_capability_header.capability_address(),
        message_address: 0,
        message_upper_address: 0,
        message_data: 0,
        mask_bits: 0,
        pending_bits: 0,
    };
    msi_capability.header = MSICapabilityHeaderData(pci_capability_header.data().0);
    let mut next_reg_address = *pci_capability_header.capability_address() + 4;
    let mut read = || {
        let value = func.read_conf_register(next_reg_address);
        next_reg_address += 4;
        value
    };
    msi_capability.message_address = read();
    if msi_capability.header.addr_64_capable() {
        msi_capability.message_upper_address = read();
    }
    msi_capability.message_data = read();
    if msi_capability.header.per_vector_mask_capable() {
        msi_capability.mask_bits = read();
        msi_capability.pending_bits = read();
    }
    Some(msi_capability)
}

fn write_msi_capability(func: &PCIFunction, msi_capability: &MSICapability) {
    let mut next_reg_address = msi_capability.capability_address;
    let mut write = |value: u32| {
        // log::debug!("Write {:02x} <= {:08x}", next_reg_address, value);
        func.write_conf_register(next_reg_address, value);
        next_reg_address += 4;
    };
    write(msi_capability.header.0);
    write(msi_capability.message_address);
    if msi_capability.header.addr_64_capable() {
        write(msi_capability.message_upper_address);
    }
    write(msi_capability.message_data);
    if msi_capability.header.per_vector_mask_capable() {
        write(msi_capability.mask_bits);
        write(msi_capability.pending_bits);
    }
}

fn configure_msi(
    func: &PCIFunction,
    message_address: u32,
    message_data: u32,
    num_vector_exponent: u8,
) -> Result<()> {
    let mut msi_capability = func
        .capability_headers()
        .find_map(|pci_capability_header| read_msi_capability(func, pci_capability_header))
        .ok_or(Error::Whatever(
            "The PCI device wasn't capable of handling MSI",
        ))?;
    msi_capability
        .header
        .set_multi_message_enable(core::cmp::min(
            num_vector_exponent,
            msi_capability.header.multi_message_capable(),
        ));
    msi_capability.header.set_msi_enable(true);
    msi_capability.message_address = message_address;
    msi_capability.message_data = message_data;
    write_msi_capability(func, &msi_capability);
    Ok(())
}

fn read_local_apic_id() -> u8 {
    let value = unsafe { core::ptr::read_volatile(0xFEE00020 as *const u32) };
    (value >> 24) as u8
}
