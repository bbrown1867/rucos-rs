//! RuCOS Kernel
//!
//! This module should only be used by the "port specific" crate (e.g. `rucos-cortex-m`).

use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicUsize, Ordering};

use crate::task::Task;

// TODO: Somehow allow user to configure this
const MAX_NUM_TASKS: usize = 32;

static CURR_TICK: AtomicU32 = AtomicU32::new(0);
static WAKE_TICK: AtomicU32 = AtomicU32::new(u32::MAX);
static CURR_TASK: AtomicUsize = AtomicUsize::new(MAX_NUM_TASKS);
static IDLE_TASK: Task = Task::new(MAX_NUM_TASKS, u8::MAX);
static TASK_TABLE: [AtomicPtr<Task>; MAX_NUM_TASKS] =
    [const { AtomicPtr::new(null_mut()) }; MAX_NUM_TASKS];

fn get_task(id: usize) -> Option<&'static Task> {
    assert!(id <= MAX_NUM_TASKS, "Invalid task");

    if id == MAX_NUM_TASKS {
        return Some(&IDLE_TASK);
    }

    let task = TASK_TABLE[id].load(Ordering::Relaxed);
    if task.is_null() {
        None
    } else {
        // Safety: Safe dereference, since we verified the pointer is not null
        unsafe { Some(&*TASK_TABLE[id].load(Ordering::Relaxed)) }
    }
}

/// Initialize the kernel
///
/// # Arguments
///
/// * `idle_stack_ptr`: Idle task stack pointer
pub fn init(idle_stack_ptr: u32) {
    IDLE_TASK.stack_ptr.store(idle_stack_ptr, Ordering::Relaxed);
}

/// Create a task
///
/// # Arguments
///
/// * `task`: Task control block
/// * `task_stack_ptr`: Task stack pointer
pub fn create(task: &'static Task, task_stack_ptr: u32) {
    assert!(task.id < MAX_NUM_TASKS, "Invalid task");

    task.stack_ptr.store(task_stack_ptr, Ordering::Relaxed);
    TASK_TABLE[task.id].store(task as *const _ as *mut _, Ordering::Relaxed);
}

/// Delete a task
///
/// # Arguments
///
/// * `id`: Task to delete
///
/// # Returns
///
/// `true` if a context switch is needed, `false` otherwise
pub fn delete(id: usize) -> bool {
    assert!(id < MAX_NUM_TASKS, "Invalid task");

    let task = get_task(id).expect("Task does not exist");
    let is_current_task = task == get_current_task().unwrap();
    TASK_TABLE[id].store(null_mut(), Ordering::Relaxed);

    is_current_task
}

/// Get the current task
///
/// # Returns
///
/// Reference to current task or `None` if the current task was deleted
///
/// # Note
///
/// If the kernel has not been started, the idle task is returned.
pub fn get_current_task() -> Option<&'static Task> {
    let id = CURR_TASK.load(Ordering::Relaxed);
    if id >= MAX_NUM_TASKS {
        Some(&IDLE_TASK)
    } else {
        get_task(id)
    }
}

/// Get the system tick
///
/// # Returns
///
/// Current value of the system tick
pub fn get_current_tick() -> u32 {
    CURR_TICK.load(Ordering::Relaxed)
}

/// Sleep the current task
///
/// # Arguments
///
/// * `delay`: Number of ticks to sleep
pub fn sleep(delay: u32) {
    assert!(delay > 0, "Delay must be non-zero");

    let curr_task = get_current_task().unwrap();
    curr_task.sleep(get_current_tick() + delay);
}

/// Suspend a task
///
/// # Arguments
///
/// * `id`: Task to suspend
///
/// # Returns
///
/// `true` if a context switch is needed, `false` otherwise
pub fn suspend(id: usize) -> bool {
    let task = get_task(id).expect("Task does not exist");
    task.suspend();
    task == get_current_task().unwrap()
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

/// Run the scheduler
///
/// # Arguments
///
/// * `curr_task_stack_ptr`: Updated stack pointer for the current task
///
/// # Returns
///
/// Stack pointer for the next task
///
/// # Note
///
/// This function should be run atomically (e.g. with interrupts disabled).
pub fn run_scheduler(curr_task_stack_ptr: u32) -> u32 {
    let curr_task = get_current_task();
    let curr_tick = get_current_tick();

    // Update current task stack pointer
    match curr_task {
        Some(task) => task.stack_ptr.store(curr_task_stack_ptr, Ordering::Relaxed),
        // Current task was deleted
        None => (),
    };

    // Idle task is always ready to run
    let mut next_task = &IDLE_TASK;

    let mut min_wake_tick = u32::MAX;

    // Find the highest priority task ready to run
    // TODO: Add time slicing (round-robin) if multiple tasks with the same priority are ready
    for i in 0..MAX_NUM_TASKS {
        match get_task(i) {
            Some(task) => {
                if task.is_sleep() {
                    let task_wake_tick = task.wake_tick();
                    if task_wake_tick <= curr_tick {
                        task.ready();
                    } else if task_wake_tick < min_wake_tick {
                        min_wake_tick = task_wake_tick;
                    }
                }

                // Remember: Lower value means higher priority
                if task.is_ready() && task.priority < next_task.priority {
                    next_task = task;
                }
            }
            None => (),
        }
    }

    WAKE_TICK.store(min_wake_tick, Ordering::Relaxed);

    if curr_task.is_none() || next_task != curr_task.unwrap() {
        CURR_TASK.store(next_task.id, Ordering::Relaxed);
        next_task.stack_ptr.load(Ordering::Relaxed)
    } else {
        curr_task_stack_ptr
    }
}

/// Update the system tick
///
/// # Arguments
///
/// * `elapsed`: Number of ticks elapsed since last call
///
/// # Returns
///
/// `true` if a context switch is needed, `false` otherwise
pub fn update_tick(elapsed: u32) -> bool {
    let curr_tick = CURR_TICK.fetch_add(elapsed, Ordering::Relaxed) + elapsed;
    let wake_tick = WAKE_TICK.load(Ordering::Relaxed);
    wake_tick <= curr_tick
}

#[cfg(test)]
mod tests {
    use super::*;

    static TASK0: Task = Task::new(0, 10);
    static TASK1: Task = Task::new(1, 11);
    static TASK2: Task = Task::new(2, 12);

    const IDLE_SP: u32 = 255;
    const TASK0_SP: u32 = 0;
    const TASK1_SP: u32 = 1;
    const TASK2_SP: u32 = 2;

    fn test_init() {
        init(IDLE_SP);
    }

    fn test_create() {
        create(&TASK0, TASK0_SP);
        create(&TASK1, TASK1_SP);
        create(&TASK2, TASK2_SP);

        assert_eq!(get_task(0).unwrap(), &TASK0);
        assert_eq!(get_task(1).unwrap(), &TASK1);
        assert_eq!(get_task(2).unwrap(), &TASK2);
        assert_eq!(get_task(3), None);
    }

    fn test_delete() {
        assert_eq!(delete(2), false);
        assert_eq!(get_task(2), None);
    }

    fn test_suspend_resume() {
        assert_eq!(suspend(0), false);
        assert!(TASK0.is_suspended());
        resume(0);
        assert!(TASK0.is_ready())
    }

    fn test_schduler() {
        // Task 0 is the highest priority task and is ready to run
        assert_eq!(run_scheduler(IDLE_SP), TASK0_SP);
        assert_eq!(get_current_task(), Some(&TASK0));
        assert_eq!(get_current_tick(), 0);

        // Sleep task 0 for 5 ticks
        sleep(5);
        assert!(TASK0.is_sleep());

        // Progress by 3 ticks
        assert_eq!(update_tick(3), false);
        assert_eq!(get_current_tick(), 3);

        // Task 1 is the highest priority task and is ready to run
        assert_eq!(run_scheduler(TASK0_SP), TASK1_SP);
        assert_eq!(get_current_task(), Some(&TASK1));
        assert_eq!(WAKE_TICK.load(Ordering::Relaxed), 5);

        // Sleep task 1 for 10 ticks
        sleep(10);
        assert!(TASK1.is_sleep());

        // All tasks sleeping (idle)
        assert_eq!(run_scheduler(TASK1_SP), IDLE_SP);
        assert_eq!(get_current_task(), Some(&IDLE_TASK));
        assert_eq!(WAKE_TICK.load(Ordering::Relaxed), 5);

        // Progress by 2 ticks
        assert_eq!(update_tick(2), true);
        assert_eq!(get_current_tick(), 5);

        // Task 0 should now be ready to run
        assert_eq!(run_scheduler(IDLE_SP), TASK0_SP);
        assert_eq!(get_current_task(), Some(&TASK0));
        assert_eq!(WAKE_TICK.load(Ordering::Relaxed), 13);
    }

    #[test]
    fn test_kernel() {
        // For simplicity, each test assumes the previous test kernel state is carried over. We
        // could reset kerenl state between each test case, but we would still need tests to run
        // without parallelization, as they are all modifying the same set of `static` variables.
        test_init();
        test_create();
        test_delete();
        test_suspend_resume();
        test_schduler();
    }
}
