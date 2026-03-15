//! RuCOS Task

use core::cmp::{self, PartialOrd};
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};

/// Task states
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskState {
    Ready = 0,
    Sleep = 1,
    Suspend = 2,
}

impl From<u8> for TaskState {
    fn from(v: u8) -> Self {
        match v {
            0 => TaskState::Ready,
            1 => TaskState::Sleep,
            2 => TaskState::Suspend,
            _ => panic!("Invalid task state"),
        }
    }
}

/// Task control block
#[derive(Debug)]
pub struct Task {
    /// Task ID
    pub id: usize,
    /// Task priority
    pub priority: u8,
    /// Task stack pointer
    pub stack_ptr: AtomicU32,
    /// Task state
    pub state: AtomicU8,
    /// Task wake tick
    pub wake_tick: AtomicU32,
}

impl Task {
    /// Create a task
    ///
    /// # Arguments
    ///
    /// * `id`: Unique ID of the task; should be in range [0:MAX_NUM_TASKS)
    /// * `priority`: Priority of the task; lower number means higher priority
    ///
    /// # Returns
    ///
    /// New task control block
    pub const fn new(id: usize, priority: u8) -> Self {
        Task {
            id: id,
            priority: priority,
            stack_ptr: AtomicU32::new(0),
            state: AtomicU8::new(TaskState::Ready as u8),
            wake_tick: AtomicU32::new(0),
        }
    }

    /// Mark the task as ready to run
    pub fn ready(&self) {
        self.state.store(TaskState::Ready as u8, Ordering::Relaxed);
    }

    /// Check if the task is ready to run
    ///
    /// # Returns
    ///
    /// `true` if the task is ready to run, `false` otherwise
    pub fn is_ready(&self) -> bool {
        TaskState::from(self.state.load(Ordering::Relaxed)) == TaskState::Ready
    }

    /// Mark the task as sleeping
    ///
    /// # Arguments
    ///
    /// * `wake_tike`: Absolute system tick to sleep until
    pub fn sleep(&self, wake_tick: u32) {
        self.wake_tick.store(wake_tick, Ordering::Relaxed);
        self.state.store(TaskState::Sleep as u8, Ordering::Relaxed);
    }

    /// Check if the task is asleep
    ///
    /// # Returns
    ///
    /// `true` if the task is asleep, `false` otherwise
    pub fn is_sleep(&self) -> bool {
        TaskState::from(self.state.load(Ordering::Relaxed)) == TaskState::Sleep
    }

    /// Mark the task as suspended
    pub fn suspend(&self) {
        self.state
            .store(TaskState::Suspend as u8, Ordering::Relaxed);
    }

    /// Check if the task is suspended
    ///
    /// # Returns
    ///
    /// `true` if the task is asleep, `false` otherwise
    pub fn is_suspended(&self) -> bool {
        TaskState::from(self.state.load(Ordering::Relaxed)) == TaskState::Suspend
    }

    /// Get the wake tick for the task
    ///
    /// # Returns
    ///
    /// Wake tick
    pub fn wake_tick(&self) -> u32 {
        self.wake_tick.load(Ordering::Relaxed)
    }
}

// Implemented to allow comparison of tasks using priority level
impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

// Implemented to allow comparison of tasks using priority level
impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.priority.cmp(&other.priority))
    }
}
