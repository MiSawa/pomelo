use core::cmp::Reverse;

use alloc::{boxed::Box, collections::binary_heap::BinaryHeap, rc::Rc};

use crate::{interrupts::InterruptIndex, prelude::*};

/// The target value of LAPIC timer frequency
pub const TARGET_FREQUENCY: u32 = 100; // once per 10 ms
const MILLISEC_PER_TICK: u64 = 1000 / TARGET_FREQUENCY as u64;

/// How much duration we will use to adjust the LAPIC timer frequency
const INITIALIZATION_MILLIS: u32 = 1000;
const MAX_TIMER_COUNT: u32 = u32::MAX;
const DEFAULT_TIMER_COUNT: u32 = 10000000;
const PM_TIMER_FREQUENCY: u32 = 3579545;

const LVT_TIMER_ADDRESS: *mut u32 = 0xFEE00320 as *mut u32;
const DIVIDE_CONFIGURATION_ADDRESS: *mut u32 = 0xFEE003E0 as *mut u32;
const INITIAL_COUNT_ADDRESS: *mut u32 = 0xFEE00380 as *mut u32;
const CURRENT_COUNT_ADDRESS: *const u32 = 0xFEE00390 as *const u32;

const DIVIDE_1_1: u32 = 0b1011;
const ONESHOT: u32 = 0b01 << 16;
const PERIODIC_INTERRUPT: u32 = 0b10 << 16;
const VECTOR: u32 = InterruptIndex::LAPICTimer as u32;

fn get_lapic_frequency(acpi2_rsdp: Option<*const core::ffi::c_void>) -> Result<u64> {
    let rsdp = acpi2_rsdp.ok_or(Error::Whatever("No ACPI RSDP"))?;
    #[derive(Copy, Clone)]
    struct Handler;
    impl acpi::AcpiHandler for Handler {
        unsafe fn map_physical_region<T>(
            &self,
            physical_address: usize,
            size: usize,
        ) -> acpi::PhysicalMapping<Self, T> {
            let virtual_start = core::ptr::NonNull::new(physical_address as *mut T).unwrap();
            acpi::PhysicalMapping::new(
                physical_address,
                virtual_start,
                physical_address,
                size,
                Handler,
            )
        }

        fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {}
    }
    let table = unsafe { acpi::AcpiTables::from_rsdp(Handler, rsdp.to_bits())? };
    let timer = table
        .platform_info()?
        .pm_timer
        .ok_or(Error::Whatever("No pm timer available"))?;

    let address = timer
        .base
        .address
        .try_into()
        .map_err(|_| Error::Whatever("The address of pm timer isn't 16 bit."))?;
    let mut pm_timer = x86_64::instructions::port::PortReadOnly::<u32>::new(address);
    const MASK: u32 = 0x00FFFFFF;
    // Use as a 24bit timer
    let mut read_time = || unsafe { pm_timer.read() & MASK };
    // Check if this actually looks like a timer
    {
        let a = read_time();
        let b = read_time();
        if a == b {
            return Err(Error::Whatever(
                "Read pm timer twice, but got the same value",
            ));
        }
    }

    let count_lapic = |f: &mut dyn FnMut()| {
        unsafe {
            core::ptr::write_volatile(DIVIDE_CONFIGURATION_ADDRESS, DIVIDE_1_1);
            core::ptr::write_volatile(LVT_TIMER_ADDRESS, ONESHOT);
            core::ptr::write_volatile(INITIAL_COUNT_ADDRESS, MAX_TIMER_COUNT);
        }
        f();
        let end = unsafe { core::ptr::read_volatile(CURRENT_COUNT_ADDRESS) };
        MAX_TIMER_COUNT - end
    };
    const WAIT_PM_TIMER_COUNT: u32 =
        (PM_TIMER_FREQUENCY as u64 * INITIALIZATION_MILLIS as u64 / 1_000) as u32;
    let lapic_count = count_lapic(&mut move || {
        let start = read_time();
        let goal = (start + WAIT_PM_TIMER_COUNT) & MASK;
        if start > goal {
            while read_time() >= start {}
        }
        while read_time() < goal {}
    });
    log::trace!("lapic count = {}", lapic_count);
    let freq = (lapic_count as u64) * 1_000 / INITIALIZATION_MILLIS as u64;
    log::trace!("LAPIC freq: {}", freq);
    Ok(freq)
}

pub fn initialize(acpi2_rsdp: Option<*const core::ffi::c_void>) {
    let timer_count = match get_lapic_frequency(acpi2_rsdp) {
        Ok(freq) => (freq / TARGET_FREQUENCY as u64) as u32,
        Err(e) => {
            log::warn!(
                "Unable to determine LAPIC frequency. Will fall back to a default. Reason: {:?}",
                e
            );
            DEFAULT_TIMER_COUNT
        }
    };
    log::info!("Set lapic count as {}", timer_count);
    unsafe {
        core::ptr::write_volatile(DIVIDE_CONFIGURATION_ADDRESS, DIVIDE_1_1);
        core::ptr::write_volatile(LVT_TIMER_ADDRESS, PERIODIC_INTERRUPT | VECTOR);
        core::ptr::write_volatile(INITIAL_COUNT_ADDRESS, timer_count);
    }
}

enum Task {
    Oneshot {
        callback: Box<dyn FnMut()>,
    },
    Periodic {
        interval_ticks: u64,
        callback: Box<dyn FnMut()>,
    },
}
struct TaskEntry {
    target_tick: u64,
    task_id: usize,
    task: Task,
}
impl PartialEq for TaskEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.target_tick, self.task_id).eq(&(other.target_tick, other.task_id))
    }
}
impl Eq for TaskEntry {}
impl PartialOrd for TaskEntry {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TaskEntry {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.target_tick, self.task_id).cmp(&(other.target_tick, other.task_id))
    }
}

pub(crate) struct Timer {
    queue: BinaryHeap<Reverse<TaskEntry>>,
    next_task_id: usize,
    tick: u64,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            next_task_id: 0,
            tick: 0,
        }
    }

    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    pub fn tick(&mut self) {
        self.tick += 1;
        while let Some(Reverse(entry)) = self.queue.peek() {
            if entry.target_tick > self.tick {
                break;
            }
            let Reverse(mut entry) = self.queue.pop().unwrap();
            match entry.task {
                Task::Oneshot { mut callback } => callback(),
                Task::Periodic {
                    interval_ticks,
                    ref mut callback,
                } => {
                    callback();
                    entry.target_tick += interval_ticks;
                    self.queue.push(Reverse(entry));
                }
            }
        }
    }

    pub fn register<F: 'static + FnOnce()>(&mut self, delay_millis: u64, f: F) {
        let mut opt = Some(f);
        self.queue.push(Reverse(TaskEntry {
            target_tick: self.tick + delay_millis / MILLISEC_PER_TICK,
            task_id: self.next_task_id,
            task: Task::Oneshot {
                callback: Box::new(move || {
                    if let Some(f) = opt.take() {
                        f();
                    }
                }),
            },
        }));
        self.next_task_id += 1;
    }

    pub fn schedule<F: 'static + FnMut()>(
        &mut self,
        initial_delay_millis: u64,
        interval_millis: u64,
        mut f: F,
    ) {
        self.queue.push(Reverse(TaskEntry {
            target_tick: self.tick + initial_delay_millis / MILLISEC_PER_TICK,
            task_id: self.next_task_id,
            task: Task::Periodic {
                interval_ticks: interval_millis / MILLISEC_PER_TICK,
                callback: Box::new(move || {
                    f();
                }),
            },
        }));
        self.next_task_id += 1;
    }
}
