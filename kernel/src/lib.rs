//! Rust Microcontroller Operating System (RuCOS)

#![cfg_attr(not(test), no_std)]

pub mod kernel;
mod task;

pub use kernel::Kernel;
