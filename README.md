# RuCOS

_Rust Microcontroller Operating System_ (RuCOS, pronounced roo-cos) is a
real-time kernel for embedded Rust applications (`no_std`).

## Design Goals

- Provide a feature set similar to [uC/OS-III](https://github.com/weston-embedded/uC-OS3) or [FreeRTOS](https://www.freertos.org/index.html)
- Easy integration: No custom build system or special project structure
- Do not use the `async`/`await` pattern
- Do not require memory management or protection hardware
- Do not use experimental language features: Compile on `stable`
- Clearly separate platform (port) specific code from the kernel

## User Guide

### Architecture

`rucos` is a collection of `no_std` functions and data structures, with a small
amount of platform specific code located in the [`port/`](src/port) directory.

### Getting Started

Using `rucos` is as simple as adding the crate to your project and calling a few APIs:

```rust
use rucos;

// ID = 42, Priority = 0 (highest priority)
static MY_TASK: rucos::Task = rucos::Task::new(42, 0);

let my_task = |_: u32| -> ! {
    loop {
        info!("Hello from Task {}", rucos::get_current_task());
        rucos::sleep(rucos::TICK_RATE_HZ);
    }
};

let mut idle_stack: [u8; IDLE_STACK_SIZE] = [0; IDLE_STACK_SIZE];
let mut my_task_stack: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

rucos::init(&mut idle_stack, None);
rucos::create(&MY_TASK, &mut my_task_stack, my_task, None);
rucos::start(...);
```

See the [`examples`](examples/) directory for more details.

## Developer Guide

### Dependencies

* Building: Rust toolchain
* Running examples: [`probe-rs`](https://probe.rs/)
* Debugging examples: [`probe-rs`](https://probe.rs/) VS Code extension

### Building

    cargo build

### Testing

Testing `rucos` requires targeting a particular device. The STM32F767 MCU
is used as the test platform, but the examples should be easy to port to
other devices.

    cargo run --example task_basic
    cargo run --example task_advanced
