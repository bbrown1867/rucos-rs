//! RuCOS kernel

use crate::task::{Task, TaskPendReason, TaskState};
use core::cmp::PartialOrd;
use core::default::Default;
use core::fmt::Debug;
use core::marker::Copy;
use core::ops::{Add, AddAssign};
use heapless::Vec;

/// Kernel
///
/// # Generics
///
/// * `SP`: The stack pointer type
/// * `TICK`: The kernel time data type, usually a numeric type
/// * `MAX_NUM_TASKS`: Upper bound on the number of tasks for the kernel
pub struct Kernel<SP, TICK, const MAX_NUM_TASKS: usize> {
    /// Kernel state
    is_running: bool,
    /// Global tick counter
    tick_counter: TICK,
    /// Task list
    task_list: Vec<Task<SP, TICK>, MAX_NUM_TASKS>,
    /// Current task ID
    curr_task_id: Option<usize>,
    /// Next task ID
    next_task_id: Option<usize>,
}

impl<SP, TICK, const MAX_NUM_TASKS: usize> Kernel<SP, TICK, MAX_NUM_TASKS>
where
    SP: Copy + Debug,
    TICK: Add<Output = TICK> + AddAssign + Copy + Debug + Default + PartialOrd,
{
    /// Initialize the kernel
    pub fn new() -> Self {
        Self {
            is_running: false,
            tick_counter: TICK::default(),
            task_list: Vec::new(),
            curr_task_id: None,
            next_task_id: None,
        }
    }

    /// Create a task
    ///
    /// # Arguments
    ///
    /// * `id`: Task ID
    /// * `priority`: Task priority, with a lower number meaning higher priority
    /// * `stack_ptr`: Task stack pointer
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    ///
    /// # Panics
    ///
    /// * The task `id` is not unique
    /// * Too many tasks have been created, more than `MAX_NUM_TASKS`
    ///
    /// # Note
    ///
    /// The kernel does not manage the task stack, caller is responsible for
    /// allocation and initialization of stack memory
    pub fn create(&mut self, id: usize, priority: usize, stack_ptr: SP) -> bool {
        // Ensure the task ID is unique
        for task in self.task_list.iter() {
            assert!(task.id != id, "The task ID is not unique");
        }

        self.task_list
            .push(Task {
                id,
                priority,
                stack_ptr,
                state: TaskState::Ready,
                pend: TaskPendReason::NotPending,
            })
            .expect("Number of tasks exceeds MAX_NUM_TASKS");

        self.scheduler()
    }

    /// Delete a task
    ///
    /// # Arguments
    ///
    /// * `id`: Task to delete or `None` to delete the current task
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    ///
    /// # Panics
    ///
    /// * The `id` provided does not correspond to a task
    /// * If called before the kernel is running
    pub fn delete(&mut self, id: Option<usize>) -> bool {
        let curr_task_idx = self.find_task_idx(self.curr_task_id.expect("Kernel not running"));
        let task_idx = match id {
            Some(id) => self.find_task_idx(id),
            None => curr_task_idx,
        };

        self.task_list.remove(task_idx);

        if curr_task_idx == task_idx {
            self.curr_task_id = None;
        }

        self.scheduler()
    }

    /// Start the kernel
    ///
    /// # Returns
    ///
    /// Stack pointer for the first task to run
    ///
    /// # Panics
    ///
    /// * No tasks have been created
    /// * The kernel is already running
    pub fn start(&mut self) -> SP {
        assert!(!self.is_running, "Kernel already running");

        self.is_running = true;

        if self.scheduler() == true {
            self.handle_context_switch(None)
        } else {
            panic!("No tasks created")
        }
    }

    /// Get the ID of the current task
    ///
    /// # Returns
    ///
    /// ID of the current task
    ///
    /// # Panics
    ///
    /// If called before the kernel is running
    pub fn get_current_task(&self) -> usize {
        self.curr_task_id.expect("Kernel not running")
    }

    /// Get the value of the global tick counter
    ///
    /// # Returns
    ///
    /// Current value of the global tick counter
    pub fn get_current_tick(&self) -> TICK {
        self.tick_counter
    }

    /// Sleep the current task
    ///
    /// # Arguments
    ///
    /// * `delay`: Number of ticks to sleep
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    ///
    /// # Panics
    ///
    /// If called before the kernel is running
    pub fn sleep(&mut self, delay: TICK) -> bool {
        let new_tick_counter = self.tick_counter + delay;
        let curr_task = self.find_task(self.curr_task_id.expect("Kernel not running"));

        curr_task.state = TaskState::Pending;
        curr_task.pend = TaskPendReason::Sleep(new_tick_counter);

        self.scheduler()
    }

    /// Suspend a task
    ///
    /// # Arguments
    ///
    /// * `id`: Task to suspend or `None` to suspend the current task
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    ///
    /// # Panics
    ///
    /// * The `id` provided does not correspond to a task
    /// * If called before the kernel is running
    pub fn suspend(&mut self, id: Option<usize>) -> bool {
        let task: &mut Task<SP, TICK> = match id {
            Some(id) => self.find_task(id),
            None => {
                let curr_task_id = self.curr_task_id.expect("Kernel not running");
                self.find_task(curr_task_id)
            }
        };

        task.state = TaskState::Pending;
        task.pend = TaskPendReason::Suspended;

        self.scheduler()
    }

    /// Resume a task
    ///
    /// # Arguments
    ///
    /// * `id`: Task to resume
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    ///
    /// # Panics
    ///
    /// The `id` provided does not correspond to a task
    pub fn resume(&mut self, id: usize) -> bool {
        let task: &mut Task<SP, TICK> = self.find_task(id);

        task.state = TaskState::Ready;
        task.pend = TaskPendReason::NotPending;

        self.scheduler()
    }

    /// Update the global tick counter
    ///
    /// # Arguments
    ///
    /// * `elapsed`: Number of ticks that have passed since last call
    ///
    /// # Returns
    ///
    /// `true` if a context switch is needed, `false` if not
    pub fn tick_update(&mut self, elapsed: TICK) -> bool {
        self.tick_counter += elapsed;

        self.scheduler()
    }

    /// Handle a context switch
    ///
    /// # Arguments
    ///
    /// * `updated_stack_ptr`: The updated stack pointer for the current task or
    ///   `None` if there is no current task
    ///
    /// # Returns
    ///
    /// The stack pointer for the next task
    ///
    /// # Panics
    ///
    /// If called when a context switch is not necessary
    pub fn handle_context_switch(&mut self, updated_stack_ptr: Option<SP>) -> SP {
        // Update current task
        match self.curr_task_id {
            Some(curr_task_id) => {
                let curr_task = self.find_task(curr_task_id);

                match updated_stack_ptr {
                    Some(sp) => curr_task.stack_ptr = sp,
                    None => (),
                };

                curr_task.state = match curr_task.state {
                    TaskState::Running => TaskState::Ready,
                    _ => curr_task.state,
                };
            }
            None => (),
        }

        // Update kernel
        let next_task_id = self.next_task_id.expect("No context switch required");
        self.curr_task_id = Some(next_task_id);
        self.next_task_id = None;

        // Update next task
        let next_task = self.find_task(next_task_id);
        next_task.state = TaskState::Running;

        // Return the next task stack pointer
        next_task.stack_ptr
    }

    fn scheduler(&mut self) -> bool {
        if !self.is_running {
            return false;
        }

        // Update pending tasks, as they might be ready to run now
        self.update_pending_tasks();

        // Update next task to run
        match self.find_highest_priority_runnable_task() {
            Some(next_task_id) => {
                match self.curr_task_id {
                    Some(curr_task_id) => {
                        // Case 1: Current task should continue running
                        if curr_task_id == next_task_id {
                            self.next_task_id = None;
                        // Case 2: Current task should be switched out
                        } else {
                            self.next_task_id = Some(next_task_id);
                        }
                    }
                    // Case 3: There is no current task (starting the kernel)
                    None => self.next_task_id = Some(next_task_id),
                }
            }
            // All tasks pending, nothing to do
            None => self.next_task_id = None,
        }

        !(self.next_task_id == None)
    }

    fn update_pending_tasks(&mut self) {
        for task in self.task_list.iter_mut() {
            match task.pend {
                TaskPendReason::Sleep(timeout) => {
                    if self.tick_counter >= timeout {
                        task.state = TaskState::Ready;
                        task.pend = TaskPendReason::NotPending;
                    }
                }
                _ => (),
            }
        }
    }

    // TODO: Assumes only one task per priority level, no round-robin scheduling
    fn find_highest_priority_runnable_task(&self) -> Option<usize> {
        let mut highest_prio_runnable_task: Option<&Task<SP, TICK>> = None;
        for task in self.task_list.iter() {
            if task.is_runnable() {
                highest_prio_runnable_task = match highest_prio_runnable_task {
                    Some(other) => {
                        if task < other {
                            Some(task)
                        } else {
                            Some(other)
                        }
                    }
                    None => Some(task),
                };
            }
        }

        match highest_prio_runnable_task {
            Some(task) => Some(task.id),
            None => None,
        }
    }

    fn find_task(&mut self, id: usize) -> &mut Task<SP, TICK> {
        self.task_list
            .iter_mut()
            .find(|t| t.id == id)
            .expect("Task does not exist")
    }

    fn find_task_idx(&self, id: usize) -> usize {
        self.task_list
            .iter()
            .position(|t| t.id == id)
            .expect("Task does not exist")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> Kernel<u32, u64, 2> {
        let mut kernel = Kernel::new();

        let mut task0_stack: [u8; 128] = [0; 128];
        kernel.create(0, 99, task0_stack.as_mut_ptr() as u32);

        let mut task1_stack: [u8; 128] = [0; 128];
        kernel.create(1, 100, task1_stack.as_mut_ptr() as u32);

        kernel.start();
        assert_eq!(kernel.curr_task_id, Some(0));
        assert_eq!(kernel.next_task_id, None);
        assert_eq!(kernel.get_current_task(), 0);

        kernel
    }

    #[test]
    fn test_sleep() {
        let mut kernel = setup();

        assert_eq!(kernel.sleep(2), true);
        assert_eq!(kernel.curr_task_id, Some(0));
        assert_eq!(kernel.next_task_id, Some(1));

        let _ = kernel.handle_context_switch(None);

        assert_eq!(kernel.curr_task_id, Some(1));
        assert_eq!(kernel.next_task_id, None);
        assert_eq!(kernel.get_current_task(), 1);

        assert_eq!(kernel.tick_update(3), true);
        assert_eq!(kernel.get_current_tick(), 3);
        assert_eq!(kernel.curr_task_id, Some(1));
        assert_eq!(kernel.next_task_id, Some(0));
    }

    #[test]
    fn test_suspend_current_task() {
        let mut kernel = setup();

        assert_eq!(kernel.suspend(None), true);
        assert_eq!(kernel.curr_task_id, Some(0));
        assert_eq!(kernel.next_task_id, Some(1));
    }

    #[test]
    fn test_suspend_other_task() {
        let mut kernel = setup();

        assert_eq!(kernel.suspend(Some(1)), false);
        assert_eq!(kernel.curr_task_id, Some(0));
        assert_eq!(kernel.next_task_id, None);
    }

    #[test]
    fn test_resume() {
        let mut kernel = setup();

        let _ = kernel.suspend(None);
        let _ = kernel.handle_context_switch(None);

        assert_eq!(kernel.resume(0), true);
        assert_eq!(kernel.curr_task_id, Some(1));
        assert_eq!(kernel.next_task_id, Some(0));
    }

    #[test]
    fn test_delete_current_task() {
        let mut kernel = setup();

        assert_eq!(kernel.delete(None), true);
        assert_eq!(kernel.curr_task_id, None);
        assert_eq!(kernel.next_task_id, Some(1));
    }

    #[test]
    fn test_delete_current_task_by_id() {
        let mut kernel = setup();

        assert_eq!(kernel.delete(Some(0)), true);
        assert_eq!(kernel.curr_task_id, None);
        assert_eq!(kernel.next_task_id, Some(1));
    }

    #[test]
    fn test_delete_other_task() {
        let mut kernel = setup();

        let _ = kernel.suspend(None);
        let _ = kernel.handle_context_switch(None);

        assert_eq!(kernel.delete(Some(0)), false);
        assert_eq!(kernel.curr_task_id, Some(1));
        assert_eq!(kernel.next_task_id, None);
    }
}
