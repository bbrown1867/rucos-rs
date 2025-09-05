# RuCOS

_Rust Microcontroller Operating System_ (RuCOS, pronounced roo-cos) is a
real-time kernel for embedded Rust applications (`no_std`).

## Design Goals

- Provide a feature set similar to [uC/OS-III](https://github.com/weston-embedded/uC-OS3) or [FreeRTOS](https://www.freertos.org/index.html)
- Easy integration: No custom build system or special project structure
- Do not use the `async`/`await` pattern
- Do not require memory management or protection hardware
- Do not use experimental language features: Compile on `stable`
- Portable: Clearly separate platform specific code from the kernel
- Tested: Thanks to portability, we can unit test the kernel on the host
- Use Rust language features to ensure memory and thread safety at compile time

## User Guide

### Architecture

The [`rucos`](kernel) crate is a collection of `no_std` data structures. It has
no platform specific or `unsafe` code. The `Kernel` struct is designed to be
used as a singleton in an embedded application.

The `rucos` crate would be difficult to use alone, as the embedded application
needs a mutable reference to the `Kernel` singleton in every task. This is
where the "port specific" crate comes in (e.g. [`rucos-cortex-m`](cortex-m)).
The port specific crate creates wrappers around the `Kernel` APIs, dealing with
platform specific details (e.g. stack initialization) and handling the `Kernel`
singleton in a safe way (e.g. disabling interrupts).

### Getting Started

Using RuCOS is as simple as adding the port specific crate to `Cargo.toml` and
calling a few APIs.

```rust
use rucos_cortex_m as rucos;

let my_task = |_: u32| -> ! {
    loop {
        info!("Hello from Task {}", rucos::get_current_task());
        rucos::sleep(rucos::TICK_RATE_HZ);
    }
};

let mut idle_stack: [u8; IDLE_STACK_SIZE] = [0; IDLE_STACK_SIZE];
let mut my_task_stack: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

rucos::init(&mut idle_stack, None);
rucos::create(0, 10, &mut my_task_stack, my_task, None);
rucos::start(...);
```

## Developer Guide

### Dependencies

* To build `rucos` and `rucos-cortex-m`, the Rust toolchain is required
* To run the `rucos-cortex-m` examples, [`probe-rs`](https://probe.rs/) is required
* To debug the `rucos-cortex-m` examples, the `probe-rs` VS Code extension is required

### Building

    ./build_all

### Testing

#### [`rucos`](kernel/)

    cd kernel && cargo test

#### [`rucos-cortex-m`](cortex-m)

Testing `rucos-cortex-m` requires targeting a particular device. The STM32F767
microcontroller is used as the test platform, but note that the example code
should be easily portable to other devices.

Ideally `cargo test` would be used to automate target testing via `defmt-test`,
but the nature of RuCOS applications is that they do not terminate and or follow
a serial sequence of steps we can assert on. Instead [`examples`](cortex-m/examples/) are used for testing and each one must be run manually:

    cd cortex-m && cargo run --example <name>
