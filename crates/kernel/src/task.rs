use core::{
    arch::asm,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering},
};

use crate::prelude::*;
use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec,
    vec::Vec,
};
use spinning_top::{MappedSpinlockGuard, Spinlock, SpinlockGuard};

lazy_static! {
    static ref TASK_MANAGER: Spinlock<Option<TaskManager>> = Spinlock::new(None);
}

pub type TaskMain = extern "sysv64" fn(u64);
pub type TaskPriority = u8;
type AtomicTaskPriority = AtomicU8;
type Generation = usize;
type AtomicGeneration = AtomicUsize;

const PREEMPTION_FREQUENCY: u32 = 50; // 20 ms
const TICKS_PER_PREEMPTION: u32 = crate::timer::TARGET_FREQUENCY / PREEMPTION_FREQUENCY;
static TICKS_UNTIL_NEXT_PREEMPTION: AtomicU32 = AtomicU32::new(0);
static TASK_CONFIG_GENERATION: AtomicGeneration = AtomicGeneration::new(0);

pub fn tick_and_check_context_switch() -> bool {
    let ret = TICKS_UNTIL_NEXT_PREEMPTION
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
            Some(prev.saturating_sub(1))
        })
        .unwrap();
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

pub fn spawn_task(task_builder: TaskBuilder) -> TaskHandle {
    with_task_manager(|mut manager| manager.add_task(task_builder)).unwrap()
}

pub fn try_switch_context() -> Result<()> {
    loop {
        let need_retry = with_task_manager(|mut manager| match manager.start_context_switch() {
            Ok(s) => {
                // log::warn!("Switching context: {:?}", s);
                s.switch(manager);
                false
            }
            Err(ContextSwitchError::NothingToRun) => {
                log::warn!("Nothing to run???");
                drop(manager);
                x86_64::instructions::interrupts::enable_and_hlt();
                true
            }
        })?;
        if !need_retry {
            break Ok(());
        }
    }
}

pub fn current_task() -> TaskHandle {
    with_task_manager(|m| m.current_handle()).unwrap()
}

fn bump_generation() {
    TASK_CONFIG_GENERATION.fetch_add(1, Ordering::SeqCst);
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
struct TaskId(usize);
impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

struct TaskHandleImpl {
    id: TaskId,
    priority: AtomicTaskPriority,
    awake: AtomicBool,
}
#[derive(Clone)]
pub struct TaskHandle {
    inner: Arc<TaskHandleImpl>,
}
impl TaskHandle {
    fn initialize(id: TaskId, priority: TaskPriority, awake: bool) -> Self {
        let inner = TaskHandleImpl {
            id,
            priority: AtomicTaskPriority::new(priority),
            awake: AtomicBool::new(awake),
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    fn id(&self) -> TaskId {
        self.inner.id
    }

    pub fn priority(&self) -> TaskPriority {
        self.inner.priority.load(Ordering::SeqCst)
    }

    pub fn awake(&self) -> bool {
        self.inner.awake.load(Ordering::SeqCst)
    }

    pub fn set_priority(&mut self, priority: TaskPriority) {
        self.inner.priority.store(priority, Ordering::SeqCst);
        bump_generation();
    }

    pub fn set_awake(&mut self, awake: bool) {
        self.inner.awake.store(awake, Ordering::SeqCst);
        bump_generation();
    }
}

type TaskContextPtr = u64;
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
#[derive(Clone, Copy, Default)]
struct Align16 {
    _data: [u8; 16],
}
#[repr(C, align(16))]
struct TaskStack {
    stack: Vec<Align16>,
}
impl TaskStack {
    fn new(size: usize) -> Self {
        Self {
            stack: vec![Align16::default(); size.div_ceil(16)],
        }
    }
    fn bottom_address(&self) -> u64 {
        self.stack.as_ptr_range().end as u64
    }
}

pub struct TaskBuilder {
    task_main: TaskMain,
    arg: u64,
    stack_size: usize,
    priority: TaskPriority,
}

impl TaskBuilder {
    #[must_use]
    pub fn new(task_main: TaskMain) -> Self {
        Self {
            task_main,
            arg: 0,
            stack_size: 128 * 1024, // 128 KiB
            priority: 1,
        }
    }

    #[must_use]
    pub fn set_arg(mut self, arg: u64) -> Self {
        self.arg = arg;
        self
    }

    #[must_use]
    pub fn set_stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = stack_size;
        self
    }
}

pub struct Task {
    context: Box<TaskContext>,
    handle: TaskHandle,
    _stack: TaskStack,
}

impl Task {
    fn empty() -> Self {
        Self::from_builder(TaskBuilder::new(_whatever).set_stack_size(0))
    }

    fn from_builder(task_builder: TaskBuilder) -> Self {
        use x86_64::instructions::segmentation::{Segment, CS, FS, GS, SS};

        let mut context = Box::new(TaskContext::default());
        let stack = TaskStack::new(task_builder.stack_size);
        context.rip = task_builder.task_main as *const u8 as u64;
        context.rdi = task_builder.arg;
        // context.rsi = 0;
        unsafe {
            asm!("mov {}, cr3", out(reg) context.cr3, options(nomem, nostack, preserves_flags))
        };
        context.rflags = 0x202; // interrupt flag
        context.cs = CS::get_reg().0 as u64; // DescriptorFlags::KERNEL_CODE64.bits();
        context.ss = SS::get_reg().0 as u64; //DescriptorFlags::KERNEL_DATA.bits();
        context.fs = FS::get_reg().0 as u64; // DescriptorFlags::KERNEL_CODE64.bits();
        context.gs = GS::get_reg().0 as u64; //DescriptorFlags::KERNEL_DATA.bits();
        context.rsp = stack.bottom_address() - 8;
        assert!(context.rsp & 0xf == 8);
        // Clear MXCSR interrupthions
        context.fxsave_area[24..28].copy_from_slice(&0x1f80u32.to_le_bytes());
        Self {
            context,
            handle: TaskHandle::initialize(TaskId::new(), task_builder.priority, true),
            _stack: stack,
        }
    }

    fn id(&self) -> TaskId {
        self.handle.id()
    }

    fn context_ptr(&self) -> TaskContextPtr {
        &*self.context as *const TaskContext as TaskContextPtr
    }
}

#[derive(Debug)]
struct ContextSwitchPartial {
    current: TaskContextPtr,
    next: TaskContextPtr,
}
#[derive(Debug)]
enum ContextSwitchError {
    NothingToRun,
    // NotNeeded,
}
impl ContextSwitchPartial {
    fn switch(self, guard: MappedSpinlockGuard<TaskManager>) {
        drop(guard);
        TICKS_UNTIL_NEXT_PREEMPTION.store(TICKS_PER_PREEMPTION, Ordering::SeqCst);
        switch_context(self.next, self.current);
    }
}

type TaskQueueEntry = (TaskHandle, TaskContextPtr);
struct TaskManager {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: VecDeque<TaskQueueEntry>,
    current_generation: Generation,
    current_task: TaskQueueEntry,
}
impl TaskManager {
    fn create() -> Self {
        let main_task = Task::empty();
        let mut tasks = BTreeMap::new();
        let handle = main_task.handle.clone();
        let ptr = main_task.context_ptr();
        tasks.insert(handle.id(), main_task);
        let old_generation = TASK_CONFIG_GENERATION.fetch_add(1, Ordering::SeqCst);
        Self {
            tasks,
            task_queue: VecDeque::new(),
            current_generation: old_generation,
            current_task: (handle, ptr),
        }
    }

    fn add_task(&mut self, task_builder: TaskBuilder) -> TaskHandle {
        let task = Task::from_builder(task_builder);
        let handle = task.handle.clone();
        assert!(
            self.tasks.insert(task.id(), task).is_none(),
            "Conflict task id???? What????"
        );
        handle
    }

    fn refresh_task_queue_if_necessary(&mut self) {
        let global_generation = TASK_CONFIG_GENERATION.load(Ordering::SeqCst);
        if global_generation <= self.current_generation {
            return;
        }
        self.task_queue.clear();
        let mut current_priority = 0;
        for task in self.tasks.values() {
            if !task.handle.awake() {
                continue;
            }
            match task.handle.priority().cmp(&current_priority) {
                core::cmp::Ordering::Less => continue,
                core::cmp::Ordering::Equal => self
                    .task_queue
                    .push_back((task.handle.clone(), task.context_ptr())),
                core::cmp::Ordering::Greater => {
                    current_priority = task.handle.priority();
                    self.task_queue.clear();
                    self.task_queue
                        .push_back((task.handle.clone(), task.context_ptr()));
                }
            }
        }
        self.current_generation = global_generation;
    }

    fn start_context_switch(
        &mut self,
    ) -> core::result::Result<ContextSwitchPartial, ContextSwitchError> {
        self.refresh_task_queue_if_necessary();
        self.task_queue.rotate_left(1);
        if let Some((handle, ptr)) = self.task_queue.front().cloned() {
            let current = self.current_task.1;
            self.current_task = (handle, ptr);
            Ok(ContextSwitchPartial { current, next: ptr })
        } else {
            Err(ContextSwitchError::NothingToRun)
        }
    }

    fn current_handle(&self) -> TaskHandle {
        self.current_task.0.clone()
    }
}

extern "sysv64" fn _whatever(_: u64) {}

#[naked]
extern "sysv64" fn switch_context(next: TaskContextPtr, current: TaskContextPtr) {
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
