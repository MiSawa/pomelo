use core::{
    arch::asm,
    marker::PhantomData,
    sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering},
};

use crate::{
    mpsc::{MPSCConsumer, MPSCProducer},
    prelude::*,
};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec,
    vec::Vec,
};
use delegate::delegate;
use spinning_top::{MappedSpinlockGuard, Spinlock, SpinlockGuard};

lazy_static! {
    static ref TASK_MANAGER: Spinlock<Option<TaskManager>> = Spinlock::new(None);
}

pub type TaskMain<T> = extern "sysv64" fn(Box<Receiver<T>>);
pub type TaskMainWithArg<T, U> = extern "sysv64" fn(Box<Receiver<T>>, Box<U>);
pub type TaskPriority = u8;
type AtomicTaskPriority = AtomicU8;
type Generation = u64;
type AtomicGeneration = AtomicU64;
type LockedManager<'a> = MappedSpinlockGuard<'a, TaskManager>;

const PREEMPTION_FREQUENCY: u32 = 50; // 20 ms
const TICKS_PER_PREEMPTION: u32 = crate::timer::TARGET_FREQUENCY / PREEMPTION_FREQUENCY;
static TICKS_UNTIL_NEXT_PREEMPTION: AtomicU32 = AtomicU32::new(0);
static TASK_CONFIG_GENERATION: AtomicGeneration = AtomicGeneration::new(0);

pub struct Receiver<T> {
    handle: TaskHandle,
    consumer: MPSCConsumer<T>,
}
impl<T> Receiver<T> {
    fn new(handle: TaskHandle) -> Self {
        Self {
            handle,
            consumer: MPSCConsumer::new(),
        }
    }
    fn producer(&self) -> MPSCProducer<T> {
        self.consumer.producer()
    }
    pub fn approximate_num_messages(&self) -> usize {
        self.consumer.approximate_len()
    }
    pub fn try_dequeue(&mut self) -> Option<T> {
        self.consumer.dequeue()
    }
    pub fn dequeue_or_wait(&mut self) -> T {
        let mut gen = self.handle.load_state();
        loop {
            if let Some(v) = self.consumer.dequeue() {
                return v;
            }
            self.handle.try_compare_and_sleep(gen);
            gen = self.handle.load_state();
        }
    }
    pub fn handle(&self) -> TypedTaskHandle<T> {
        TypedTaskHandle {
            inner: self.handle.clone(),
            producer: self.consumer.producer(),
        }
    }
}

pub fn initialize<T>() -> (Receiver<T>, TypedTaskHandle<T>) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut task_manager = TASK_MANAGER.lock();
        if task_manager.is_some() {
            panic!("Initializing task manager more than once!");
        }
        let (manager, receiver, handle) = TaskManager::create();
        *task_manager = Some(manager);
        (receiver, handle)
    })
}

pub fn tick_and_check_context_switch() -> bool {
    let ret = TICKS_UNTIL_NEXT_PREEMPTION
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |prev| {
            Some(prev.saturating_sub(1))
        })
        .unwrap();
    ret == 0
}

fn with_task_manager<T, F: FnOnce(LockedManager) -> T>(f: F) -> Result<T> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if let Some(locked) = TASK_MANAGER.try_lock() {
            let manager = SpinlockGuard::try_map(locked, Option::as_mut)
                .map_err(|_| Error::Whatever("Task manager is not initialized yet"))?;
            Ok(f(manager))
        } else {
            Err(Error::Whatever(
                "Failed to take the lock for task schedular...",
            ))
        }
    })
}

pub fn spawn_task<T>(task_builder: TaskBuilder<T>) -> TypedTaskHandle<T> {
    with_task_manager(|mut manager| manager.add_task(task_builder)).unwrap()
}

pub fn try_switch_context() -> Result<()> {
    loop {
        let need_retry = with_task_manager(|mut manager| match manager.start_context_switch() {
            Ok(s) => {
                log::trace!("Switching context: {:?}", s);
                s.switch(manager);
                false
            }
            Err(ContextSwitchError::NothingToRun) => {
                log::error!("Nothing to run");
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

fn bump_global_generation() {
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
    name: &'static str,
    priority: AtomicTaskPriority,
    /// generation and waking flag
    /// Bit 0 corresponds to the waking flag, and
    /// the other 63 bits correspond to the generation.
    state: AtomicU64,
}
#[derive(Clone)]
pub struct TaskHandle {
    inner: Arc<TaskHandleImpl>,
}
impl TaskHandle {
    fn initialize(id: TaskId, name: &'static str, priority: TaskPriority, waking: bool) -> Self {
        let state = if waking { 1 } else { 0 };
        let inner = TaskHandleImpl {
            id,
            name,
            priority: AtomicTaskPriority::new(priority),
            state: AtomicU64::new(state),
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

    pub fn waking(&self) -> bool {
        self.inner.state.load(Ordering::SeqCst) & 1 == 1
    }

    pub fn set_priority(&self, priority: TaskPriority) {
        self.inner.priority.store(priority, Ordering::SeqCst);
        bump_global_generation();
    }

    pub fn set_waking(&self, waking: bool) {
        if waking {
            self.awake();
        } else {
            self.put_sleep();
        }
    }

    pub fn awake(&self) {
        self.inner
            .state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some((s + 2) | 1))
            .ok();
        bump_global_generation();
    }

    pub fn put_sleep(&self) {
        self.inner
            .state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |s| Some((s + 2) & !1))
            .ok();
        bump_global_generation();
    }

    pub fn load_state(&self) -> u64 {
        self.inner.state.load(Ordering::SeqCst)
    }

    pub fn try_compare_and_sleep(&self, state: u64) -> bool {
        let success = self
            .inner
            .state
            .compare_exchange(state, (state + 2) & !1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok();
        if success {
            x86_64::instructions::interrupts::enable();
            bump_global_generation();
        }
        success
    }
}

pub struct TypedTaskHandle<T> {
    inner: TaskHandle,
    producer: MPSCProducer<T>,
}
impl<T> Clone for TypedTaskHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            producer: self.producer.clone(),
        }
    }
}
impl<T> TypedTaskHandle<T> {
    delegate! {
        to self.inner {
            pub fn priority(&self) -> TaskPriority;
            pub fn waking(&self) -> bool;
            pub fn set_priority(&self, priority: TaskPriority);
            pub fn set_waking(&self, waking: bool);
            pub fn awake(&self);
            pub fn put_sleep(&self);
            pub fn load_state(&self) -> u64;
            pub fn try_compare_and_sleep(&self, state: u64) -> bool;
        }
    }

    pub fn send(&self, value: T) {
        log::trace!("Sending value to {}", self.inner.inner.name);
        self.producer.enqueue(value);
        self.inner.awake();
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

pub struct TaskBuilder<T> {
    name: &'static str,
    task_main: u64,
    arg: u64,
    stack_size: usize,
    priority: TaskPriority,
    _phantom: PhantomData<T>,
}

impl<T> TaskBuilder<T> {
    #[must_use]
    pub fn new(name: &'static str, task_main: TaskMain<T>) -> Self {
        Self {
            name,
            task_main: task_main as *const u8 as u64,
            arg: 0,
            stack_size: 512 * 1024, // 128 KiB
            priority: 5,
            _phantom: Default::default(),
        }
    }

    #[must_use]
    pub fn new_with_arg<U>(
        name: &'static str,
        task_main: TaskMainWithArg<T, U>,
        arg: Box<U>,
    ) -> Self {
        Self {
            name,
            task_main: task_main as *const u8 as u64,
            arg: Box::into_raw(arg) as u64,
            stack_size: 128 * 1024, // 128 KiB
            priority: 5,
            _phantom: Default::default(),
        }
    }

    #[must_use]
    pub fn set_stack_size(mut self, stack_size: usize) -> Self {
        self.stack_size = stack_size;
        self
    }

    #[must_use]
    pub fn set_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }
}

pub struct Task {
    context: Box<TaskContext>,
    handle: TaskHandle,
    _stack: TaskStack,
}

impl Task {
    fn empty<T>() -> (Self, Receiver<T>, TypedTaskHandle<T>) {
        let handle = TaskHandle::initialize(TaskId::new(), "main", 10, true);
        let receiver = Receiver::new(handle.clone());
        let producer = receiver.producer();
        let context = Box::new(TaskContext::default());
        (
            Self {
                context,
                handle: handle.clone(),
                _stack: TaskStack::new(0),
            },
            receiver,
            TypedTaskHandle {
                inner: handle,
                producer,
            },
        )
    }
    fn create_with_handle<T>(task_builder: TaskBuilder<T>) -> (Self, TypedTaskHandle<T>) {
        use x86_64::instructions::segmentation::{Segment, CS, FS, GS, SS};

        let handle = TaskHandle::initialize(
            TaskId::new(),
            task_builder.name,
            task_builder.priority,
            true,
        );
        let receiver = Box::new(Receiver::new(handle.clone()));
        let producer = receiver.producer();

        let mut context = Box::new(TaskContext::default());
        let stack = TaskStack::new(task_builder.stack_size);
        context.rip = task_builder.task_main;
        context.rdi = Box::into_raw(receiver) as u64;
        context.rsi = task_builder.arg;
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
        (
            Self {
                context,
                handle: handle.clone(),
                _stack: stack,
            },
            TypedTaskHandle {
                inner: handle,
                producer,
            },
        )
    }

    fn id(&self) -> TaskId {
        self.handle.id()
    }

    fn context_ptr(&self) -> TaskContextPtr {
        &*self.context as *const TaskContext as TaskContextPtr
    }
}

#[derive(Debug)]
#[allow(unused)] // We do use this via {:?} on debug
struct ContextSwitchPartial {
    current_name: &'static str,
    current: TaskContextPtr,
    next_name: &'static str,
    next: TaskContextPtr,
}
#[derive(Debug)]
enum ContextSwitchError {
    NothingToRun,
    // NotNeeded,
}
impl ContextSwitchPartial {
    fn switch(self, guard: LockedManager) {
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
    fn create<T>() -> (Self, Receiver<T>, TypedTaskHandle<T>) {
        let (main_task, receiver, typed_handle) = Task::empty();
        let mut tasks = BTreeMap::new();
        let handle = main_task.handle.clone();
        let ptr = main_task.context_ptr();
        tasks.insert(handle.id(), main_task);
        let old_generation = TASK_CONFIG_GENERATION.fetch_add(1, Ordering::SeqCst);
        let mut ret = Self {
            tasks,
            task_queue: VecDeque::new(),
            current_generation: old_generation,
            current_task: (handle, ptr),
        };
        ret.add_task(TaskBuilder::new("idle", idle_task_main));
        (ret, receiver, typed_handle)
    }

    fn add_task<T>(&mut self, task_builder: TaskBuilder<T>) -> TypedTaskHandle<T> {
        let (task, handle) = Task::create_with_handle(task_builder);
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
            if !task.handle.waking() {
                continue;
            }
            log::trace!(
                "Task {}: {}",
                task.handle.inner.name,
                task.handle.inner.state.load(Ordering::SeqCst)
            );
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
        if !self.task_queue.is_empty() {
            self.task_queue.rotate_left(1);
        }
        if let Some((handle, ptr)) = self.task_queue.front().cloned() {
            let current_name = self.current_task.0.inner.name;
            let current = self.current_task.1;
            let next_name = handle.inner.name;
            self.current_task = (handle, ptr);
            Ok(ContextSwitchPartial {
                current,
                current_name,
                next: ptr,
                next_name,
            })
        } else {
            Err(ContextSwitchError::NothingToRun)
        }
    }

    fn current_handle(&self) -> TaskHandle {
        self.current_task.0.clone()
    }
}

extern "sysv64" fn _whatever<T>(_: Box<Receiver<T>>) {}

extern "sysv64" fn idle_task_main(_receiver: Box<Receiver<()>>) {
    loop {
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

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
