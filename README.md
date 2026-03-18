# RuCOS

_Rust Microcontroller Operating System_ (RuCOS, pronounced roo-cos) is a
real-time kernel for embedded Rust applications (`no_std`).

## Design Goals

- Provide a feature set similar to [uC/OS-III](https://github.com/weston-embedded/uC-OS3) or [FreeRTOS](https://www.freertos.org/index.html)
- Easy integration: No custom build system or special project structure
- Do not use the `async`/`await` pattern
- Do not require memory management or protection hardware
- Do not use experimental language features: Compile on `stable`
- Portable: Clearly separate platform specific and generic code
- Tested: Thanks to portability, we can unit test the kernel on the host

## User Guide

### Architecture

The [`rucos`](kernel) crate is a collection of `no_std` data structures and
a very simple scheduler. It has no platform specific code or and almost no
`unsafe` code. Aside to the `Task` data strcuture, the `rucos` crate is not
designed to be directly used by end users. Instead, a "port specific" crate,
like [`rucos-cortex-m`](cortex-m) should be used. The port specific crate
handles architecture details like stack initialization and context switching,
and ensures scheduling is done in a safely (e.g. disabling interrupts).

### Getting Started

Using RuCOS is as simple as adding the port specific crate to `Cargo.toml` and
calling a few APIs.

```rust
use rucos_cortex_m as rucos;

// ID = 6, Priority = 0 (highest priority)
static MY_TASK: rucos::Task = rucos::Task::new(6, 0);

let my_task = |_: u32| -> ! {
    loop {
        info!("Hello from Task {}", rucos::get_current_task());
        rucos::sleep(TICK_RATE_HZ);
    }
};

let idle_stack: [u8; IDLE_STACK_SIZE] = [0; IDLE_STACK_SIZE];
let my_task_stack: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

rucos::init(&idle_stack, None);
rucos::create(&MY_TASK, &my_task_stack, my_task, None);
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
