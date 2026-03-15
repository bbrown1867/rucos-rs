//! RuCOS Kernel

#![no_std]

mod port;
mod task;

use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use port::{port_init_stack, port_start, port_switch_context};
pub use task::Task;

// TODO: Allow crate user to configure this. This is also the
// idle task ID, since other IDs are reserved for user tasks.
const MAX_NUM_TASKS: usize = 16;

static SYS_TICK: AtomicU32 = AtomicU32::new(0);
static CURR_TASK: AtomicUsize = AtomicUsize::new(MAX_NUM_TASKS);
static IDLE_TASK: Task = Task::new(MAX_NUM_TASKS, u8::MAX);
static TASKS: [AtomicPtr<Task>; MAX_NUM_TASKS] =
    [const { AtomicPtr::new(null_mut()) }; MAX_NUM_TASKS];

fn idle_task(_: u32) -> ! {
    loop {}
}

fn get_task(id: usize) -> Option<&'static Task> {
    if id == MAX_NUM_TASKS {
        return Some(&IDLE_TASK);
    } else if id > MAX_NUM_TASKS {
        panic!("Too many tasks");
    }

    let task = TASKS[id].load(Ordering::Relaxed);
    if task.is_null() {
        None
    } else {
        // Safety: Safe dereference, since we verified the pointer is not null
        unsafe { Some(&*TASKS[id].load(Ordering::Relaxed)) }
    }
}

fn find_highest_priority_ready_task() -> &'static Task {
    // Idle task is always ready to run
    let mut winner = &IDLE_TASK;

    for i in 0..MAX_NUM_TASKS {
        match get_task(i) {
            Some(task) => {
                // Remember: Lower value means higher priority
                if task.is_ready() && task.priority < winner.priority {
                    winner = task;
                }
            }
            None => (),
        }
    }

    winner
}

/// Initialize the kernel
///
/// # Arguments
///
/// * `idle_stack`: Idle task stack
/// * `user_idle_task`: Optional idle task function
///
/// # Note
///
/// The idle task is the lowest priority task and is always ready to run, it
/// must not block or call any kernel APIs (e.g. `sleep`).
pub fn init(idle_stack: &mut [u8], user_idle_task: Option<fn(u32) -> !>) {
    let idle_stack_ptr = port_init_stack(idle_stack, user_idle_task.unwrap_or(idle_task), None);
    IDLE_TASK.stack_ptr.store(idle_stack_ptr, Ordering::Relaxed);
}

/// Start the kernel
///
/// # Arguments
///
/// * `clock_freq_hz`: Core clock frequency in hertz
///
/// # Note
///
/// Does not return: Program execution continues from tasks or interrupt
/// handlers after calling this API.
pub fn start(clock_freq_hz: u32) -> ! {
    // For simplicity, use the idle task as the first stack pointer, it will
    // be preempted after the first tick by another task the user added.
    let first_task_stack_ptr = IDLE_TASK.stack_ptr.load(Ordering::Relaxed);
    port_start(first_task_stack_ptr, clock_freq_hz);
}

/// Create a task
///
/// # Arguments
///
/// * `task`: Task control block
/// * `stack`: Task stack memory
/// * `entry`: Task function
/// * `arg`: An optional argument to pass to `entry`
pub fn create(task: &'static Task, stack: &mut [u8], entry: fn(u32) -> !, arg: Option<u32>) {
    assert!(task.id < MAX_NUM_TASKS, "Too many tasks");

    let task_stack_ptr = port_init_stack(stack, entry, arg);
    task.stack_ptr.store(task_stack_ptr, Ordering::Relaxed);

    TASKS[task.id].store(task as *const _ as *mut _, Ordering::Relaxed);
}

/// Delete a task
///
/// # Arguments
///
/// * `id`: Task to delete
pub fn delete(id: usize) {
    assert!(id < MAX_NUM_TASKS, "Too many tasks");

    let task = get_task(id).expect("Task does not exist");
    TASKS[id].store(null_mut(), Ordering::Relaxed);

    if task == get_current_task() {
        port_switch_context();
    }
}

/// Get the current task
///
/// # Returns
///
/// Reference to current task
pub fn get_current_task() -> &'static Task {
    get_task(CURR_TASK.load(Ordering::Relaxed)).unwrap()
}

/// Get the system tick
///
/// # Returns
///
/// Current value of the system tick
///
/// # Note
///
/// Ticks correspond to system time based platform timer configuration.
pub fn get_current_tick() -> u32 {
    SYS_TICK.load(Ordering::Relaxed)
}

/// Sleep the current task
///
/// # Arguments
///
/// * `delay`: Number of ticks to sleep
///
/// # Note
///
/// Ticks correspond to system time based platform timer configuration.
pub fn sleep(delay: u32) {
    if delay > 0 {
        let curr_task = get_current_task();
        curr_task.sleep(get_current_tick() + delay);
        port_switch_context();
    }
}

/// Suspend a task
///
/// # Arguments
///
/// * `id`: Task to suspend
pub fn suspend(id: usize) {
    let task = get_task(id).expect("Task does not exist");
    task.suspend();
    if task == get_current_task() {
        port_switch_context();
    }
}

/// Resume a task
///
/// # Arguments
///
/// * `id`: Task to resume
pub fn resume(id: usize) {
    let task = get_task(id).expect("Task does not exist");
    task.ready();
}

/// Platform agnostic context switch implementation
///
/// # Arguments
///
/// * `curr_task_stack_ptr`: Stack pointer of the current task
///
/// # Returns
///
/// Stack pointer of the next task
///
/// # Safety
///
/// Runs with interrupts disabled on a single-core microcontroller
#[no_mangle]
pub unsafe fn on_context_switch(curr_task_stack_ptr: u32) -> u32 {
    let curr_task = get_current_task();
    curr_task
        .stack_ptr
        .store(curr_task_stack_ptr, Ordering::Relaxed);

    let next_task = find_highest_priority_ready_task();

    if next_task != curr_task {
        CURR_TASK.store(next_task.id, Ordering::Relaxed);
        next_task.stack_ptr.load(Ordering::Relaxed)
    } else {
        curr_task_stack_ptr
    }
}

/// Platform agnostic system tick implementation
pub fn on_system_tick() {
    let prev_tick = SYS_TICK.fetch_add(1, Ordering::Relaxed);
    let curr_tick = prev_tick + 1;

    for i in 0..MAX_NUM_TASKS {
        match get_task(i) {
            Some(task) => {
                if task.is_sleep() && task.wake_tick() <= curr_tick {
                    task.ready();
                }
            }
            None => (),
        }
    }

    let next_task = find_highest_priority_ready_task();

    if next_task != get_current_task() {
        port_switch_context();
    }
}
