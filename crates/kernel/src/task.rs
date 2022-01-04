use core::{arch::asm, sync::atomic::{AtomicU32, Ordering}};

use crate::prelude::*;
use alloc::{boxed::Box, vec, vec::Vec};
use spinning_top::{MappedSpinlockGuard, Spinlock, SpinlockGuard};

lazy_static! {
    static ref TASK_MANAGER: Spinlock<Option<TaskManager>> = Spinlock::new(None);
}

pub type TaskMain = extern "sysv64" fn(u64);
const PREEMPTION_FREQUENCY: u32 = 50; // 20 ms
const TICKS_PER_PREEMPTION: u32 = crate::timer::TARGET_FREQUENCY / PREEMPTION_FREQUENCY;
static TICKS_UNTIL_NEXT_PREEMPTION: AtomicU32 = AtomicU32::new(0);

pub fn tick_and_check_context_switch() -> bool {
    let ret = TICKS_UNTIL_NEXT_PREEMPTION.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
        Some(prev.saturating_sub(1))
    }).unwrap();
    ret == 0
}

fn with_task_manager<T, F: FnOnce(MappedSpinlockGuard<TaskManager>) -> T>(f: F) -> Result<T> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(locked) = TASK_MANAGER.try_lock() {
            let manager =
                SpinlockGuard::map(locked, |opt| opt.get_or_insert_with(TaskManager::create));
            Ok(f(manager))
        } else {
            Err(Error::Whatever(
                "Failed to take the lock for task schedular...",
            ))
        }
    })
}

pub fn spawn_task(main: TaskMain, arg: u64) {
    with_task_manager(|mut manager| {
        manager.add_task(main, arg);
    })
    .unwrap();
}

pub fn try_switch_context() -> Result<()> {
    with_task_manager(|mut manager| {
        let (next, current) = manager.start_context_switch();
        drop(manager);
        TICKS_UNTIL_NEXT_PREEMPTION.store(TICKS_PER_PREEMPTION, Ordering::SeqCst);
        switch_context(next, current);
    })
}

#[repr(C, align(16))]
#[derive(Debug)]
struct TaskContext {
    // Offset 0x00
    cr3: u64,
    rip: u64,
    rflags: u64,
    reserved1: u64,
    // Offset 0x20
    cs: u64,
    ss: u64,
    fs: u64,
    gs: u64,
    // Offset 0x40
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    // Offset 0x60
    rdi: u64,
    rsi: u64,
    rsp: u64,
    rbp: u64,
    // Offset 0x80
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    // Offset 0xA0
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    // Offset 0xC0
    fxsave_area: [u8; 512],
}
impl Default for TaskContext {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C, align(16))]
struct TaskStack {
    _stack: [u64; 1024],
}
impl Default for TaskStack {
    fn default() -> Self {
        Self { _stack: [0; 1024] }
    }
}
impl TaskStack {
    fn bottom_address(&self) -> u64 {
        self._stack.as_ptr_range().end as u64
    }
}

pub struct Task {
    context: Box<TaskContext>,
    _stack: Box<TaskStack>,
}

impl Task {
    fn empty() -> Self {
        Self {
            context: Box::new(TaskContext::default()),
            _stack: Box::new(TaskStack::default()),
        }
    }
    fn new(task_main: TaskMain, arg: u64) -> Self {
        use x86_64::instructions::segmentation::{Segment, CS, SS};

        let mut context = Box::new(TaskContext::default());
        let stack = Box::new(TaskStack::default());
        context.rip = task_main as *const u8 as u64;
        context.rdi = arg;
        // context.rsi = 0;
        unsafe {
            asm!("mov {}, cr3", out(reg) context.cr3, options(nomem, nostack, preserves_flags))
        };
        context.rflags = 0x202; // interrupt flag
        context.cs = CS::get_reg().0 as u64; // DescriptorFlags::KERNEL_CODE64.bits();
        context.ss = SS::get_reg().0 as u64; //DescriptorFlags::KERNEL_DATA.bits();
        context.rsp = stack.bottom_address() - 8;
        assert!(context.rsp & 0xf == 8);
        // Clear MXCSR interrupthions
        context.fxsave_area[24..28].copy_from_slice(&0x1f80u32.to_le_bytes());
        Self {
            context,
            _stack: stack,
        }
    }

    pub fn context_ptr(&self) -> u64 {
        &*self.context as *const TaskContext as u64
        // (MAIN_CONTEXT.as_ptr() as u64).next_multiple_of(32)
    }
}

struct TaskManager {
    tasks: Vec<Task>,
    current_task: usize,
}
impl TaskManager {
    fn create() -> Self {
        Self {
            tasks: vec![Task::empty()],
            current_task: 0,
        }
    }

    fn add_task(&mut self, task_main: TaskMain, arg: u64) {
        self.tasks.push(Task::new(task_main, arg));
    }

    fn start_context_switch(&mut self) -> (u64, u64) {
        let current = self.tasks[self.current_task].context_ptr();
        self.current_task = (self.current_task + 1) % self.tasks.len();
        let next = self.tasks[self.current_task].context_ptr();
        (next, current)
    }
}

#[naked]
extern "sysv64" fn switch_context(next: u64, current: u64) {
    unsafe {
        asm! {
            "mov [rsi + 0x40], rax",
            "mov [rsi + 0x48], rbx",
            "mov [rsi + 0x50], rcx",
            "mov [rsi + 0x58], rdx",
            "mov [rsi + 0x60], rdi",
            "mov [rsi + 0x68], rsi",

            "lea rax, [rsp + 8]",
            "mov [rsi + 0x70], rax", // RSP
            "mov [rsi + 0x78], rbp",
            "mov [rsi + 0x80], r8",
            "mov [rsi + 0x88], r9",
            "mov [rsi + 0x90], r10",
            "mov [rsi + 0x98], r11",
            "mov [rsi + 0xa0], r12",
            "mov [rsi + 0xa8], r13",
            "mov [rsi + 0xb0], r14",
            "mov [rsi + 0xb8], r15",

            "mov rax, cr3",
            "mov [rsi + 0x00], rax", //  CR3
            "mov rax, [rsp]",
            "mov [rsi + 0x08], rax", //  RIP
            "pushfq",
            "pop qword ptr [rsi + 0x10]", // RFLAGS

            "mov ax, cs",
            "mov [rsi + 0x20], rax",
            "mov bx, ss",
            "mov [rsi + 0x28], rbx",
            "mov cx, fs",
            "mov [rsi + 0x30], rcx",
            "mov dx, gs",
            "mov [rsi + 0x38], rdx",

            "fxsave [rsi + 0xc0]",

            // // stack frame for iret
            // "push qword ptr [rsi + 0x28]", // SS
            // "push qword ptr [rsi + 0x70]", // RSP
            // "push qword ptr [rsi + 0x10]", // RFLAGS
            // "push qword ptr [rsi + 0x20]", // CS
            // "push qword ptr [rsi + 0x08]", // RIP

            // stack frame for iret
            "push qword ptr [rdi + 0x28]", // SS
            "push qword ptr [rdi + 0x70]", // RSP
            "push qword ptr [rdi + 0x10]", // RFLAGS
            "push qword ptr [rdi + 0x20]", // CS
            "push qword ptr [rdi + 0x08]", // RIP

            // restore context
            "fxrstor [rdi + 0xc0]",

            "mov rax, [rdi + 0x00]",
            "mov cr3, rax",
            "mov rax, [rdi + 0x30]",
            "mov fs, ax",
            "mov rax, [rdi + 0x38]",
            "mov gs, ax",

            "mov rax, [rdi + 0x40]",
            "mov rbx, [rdi + 0x48]",
            "mov rcx, [rdi + 0x50]",
            "mov rdx, [rdi + 0x58]",
            "mov rsi, [rdi + 0x68]",
            "mov rbp, [rdi + 0x78]",
            "mov r8,  [rdi + 0x80]",
            "mov r9,  [rdi + 0x88]",
            "mov r10, [rdi + 0x90]",
            "mov r11, [rdi + 0x98]",
            "mov r12, [rdi + 0xa0]",
            "mov r13, [rdi + 0xa8]",
            "mov r14, [rdi + 0xb0]",
            "mov r15, [rdi + 0xb8]",

            "mov rdi, [rdi + 0x60]",

            "iretq",
            options(noreturn)
        }
    }
}
