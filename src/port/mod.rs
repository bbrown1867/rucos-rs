//! RuCOS Ports

#[cfg(target_arch = "arm")]
pub mod cortex_m;

#[cfg(target_arch = "arm")]
pub use cortex_m::{port_init_stack, port_start, port_switch_context};
