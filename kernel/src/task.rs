//! RuCOS Task

use core::cmp::{Ordering, PartialOrd};

/// Task states
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskState {
    /// Waiting for some event
    Pending,
    /// Ready to run
    Ready,
    /// Currently running
    Running,
}

/// Task pend reasons and associated pend data
///
/// # Generics
///
/// * `TICK`: The kernel time data type, usually a numeric type
#[derive(Debug)]
pub enum TaskPendReason<TICK> {
    /// The task is not pending
    NotPending,
    /// The task is suspended
    Suspended,
    /// The task is sleeping until some tick count in the future
    Sleep(TICK),
}

/// Task control block
///
/// # Generics
///
/// * `SP`: The stack pointer type
/// * `TICK`: The kernel time data type, usually a numeric type
#[derive(Debug)]
pub struct Task<SP, TICK> {
    /// Task ID
    pub id: usize,
    /// Task priority
    pub priority: usize,
    /// Task stack pointer
    pub stack_ptr: SP,
    /// Task state
    pub state: TaskState,
    /// Task pend reason
    pub pend: TaskPendReason<TICK>,
}

/// Allow comparison of tasks using priority level
impl<SP, TICK> PartialEq for Task<SP, TICK> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

/// Allow comparison of tasks using priority level
impl<SP, TICK> PartialOrd for Task<SP, TICK> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.priority.cmp(&other.priority))
    }
}

impl<SP, TICK> Task<SP, TICK> {
    /// Check if the task is runnable
    ///
    /// # Returns
    ///
    /// `true` if the task is runnable, `false` if not
    pub fn is_runnable(&self) -> bool {
        self.state == TaskState::Ready || self.state == TaskState::Running
    }
}
