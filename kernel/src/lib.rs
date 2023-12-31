//! Rust Microcontroller Operating System (RuCOS) Kernel

#![cfg_attr(not(test), no_std)]

pub mod kernel;
mod task;

pub use kernel::Kernel;
