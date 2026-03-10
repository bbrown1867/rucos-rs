# RuCOS Cortex-M

A port of the RuCOS [kernel](../kernel) to Cortex-M.

## Implementation Notes

### References

- _The Definitive Guide to Cortex-M3 and Cortex-M4 Processors_ by Joseph Yiu
- _ARM Cortex-M RTOS Context Switching_ by Chris Coleman

### Architecture

- Thumb instruction set
- ARMv7-M: Cortex-M3
- ARMv7-EM: Cortex-M4, Cortex-M7
    - Adds DSP instructions
    - Adds an optional floating point unit (FPU)
    - Cortex-M7 FPU can be double-precision

### Modes

- Priviledged access: Can access all system resources no restrictions
    - All Cortex-M devices start with this
- Unpriviledged access: Some memory regions and special instructions unavailable
    - Not available on Cortex-M0
- Handler mode:
    - Used when executing exception handlers
    - Always has privileged access level
- Thread mode:
    - Normal application code
    - Can be priviledged or unpriviledged, depending on `CONTROL` register

### Register Bank

- 16 registers
- `R0 - R12`: General purpose
- `R13`: Stack pointer (`SP`)
    - Used for accessing stack memory via `PUSH` and `POP` instructions
    - `MSP`: Main stack pointer, used on reset and in handler mode
    - `PSP`: Process stack pointer, can only be used in thread mode
- `R14`: Link register (`LR`)
- `R15`: Program counter (`PC`)

### Special Registers

Accessed with special assembly instructions:

```
MRS <reg>        , <special_reg>; Read special register into register
MSR <special_reg>, <reg>        ; Write to special register
```

Description of the special registers:

- Program Status Registers: `xPSR`
    - Application, Execution, and Interrupt PSR
    - Presented as three separate registers, but can be accessed as one (`PSR`)
    - Mostly contains instruction set flags (e.g. negative, carry, overflow)
    - Also contains the exception number, when in handler mode
- Exception or interrupt masking: `PRIMASK`, `FAULTMASK`, `BASEPRI`
    - All of these fields default to zero
    - `PRIMASK` (1b): Blocks all interrupts/exceptions, except HardFault + NMI
    - `FAULTMASK` (1b): Similar to PRIMASK, but also blocks HardFault
    - `BASEPRI` (n-bits):
        - Blocks all exceptions with same or lower priority level (0: disabled)
        - Width depends on number of priority levels, which is MCU specific
    - In addition to `MRS` / `MSR`, can use `CPS` instructions as well
        - `CPSIE i`, `CPSID i`: Clear or set PRIMASK
        - `CPSIE f`, `CPSID f`: Clear or set FAULTMASK
- Control register: `CONTROL`
    - 3 fields, each 1-bit wide:
        - `SPSEL`: Selection of stack pointer (`MSP` or `PSP`)
        - `nPRIV`: Access level in thread mode (priviledged or unpriviledged)
        - `FPCA`: Use FPU in current context (code currently executing)
    - `FPCA` not applicable to Cortex-M3
    - Can only write this register with priviledged access
    - Recommended to execute an `ISB` instruction after modification

### Floating Point Unit (FPU)

- Multiple floating-point extension options for Cortex-M: FPv4-SP and FPv5
- For both, there are 32 32-bit (single-precision) registers (`S0 - S31`)
    - Can also be addressed as 16 64-bit (double-precision) registers
- Additional special regsiter: `FPSCR`
    - Similar to `xPSR` but for floating-point operations
- Even if the MCU has an FPU, when the device is reset it is disabled
    - Must write to coprocessor access control register (`CPACR`) to enable it
    - Two bits in `CPACR` also control FPU access (priviledged or unpriviledged)

### Stacking

- Registers must be preserved across function calls or interrupts
    - This is done using the stack
- Registers pushed on entry, popped off on exit
    - This process is called "stacking"
- Hardware automatically stacks `R0 - R3`, `R12`, `LR`, `PC`, and `xPSR`
    - When using an FPU, this also includes `S0 - S15` and `FPSCR` (extended)
- Software is responsible for stacking `R4 - R8`, `R11`, and `SP`
    - When using an FPU, this also includes `S16 - S31` (extended)
- To restore state on exit, a special `EXC_RETURN` value is loaded into `LR`
    - `0xFFFF_FFF1`: Return to handler mode using MSP
    - `0xFFFF_FFF9`: Return to thread mode using MSP
    - `0xFFFF_FFFD`: Return to thread mode using PSP
    - `0xFFFF_FFE1`: Return to handler mode using MSP (FPU extended frame)
    - `0xFFFF_FFE9`: Return to thread mode using MSP (FPU extended frame)
    - `0xFFFF_FFED`: Return to thread mode using PSP (FPU extended frame)

### Ownership Model

#### Review

Rust ownership model for references:
- One mutable reference or multiple immutable references, but not both
- References must always be valid

Interior mutability:
- `Box<T>` - Heap-allocated with ownership model enforced at compile-time
- `RefCell<T>`- Moves ownership model enforcement to run-time
    - Will `panic` if violated
    - Only can be used in single-threaded context, one data owner
    - Useful when you know your code follows borrowing rules, but compiler doesn't
- `Rc<T>` should replace `RefCell<T>` if there are multiple owners (ref counting)
- `Arc<T>` should replace `Rc<T>` in multi-threaded context (atomic ref counting)

#### Challenge

`Kernel` is a singleton and `static mut`, which requires `unsafe` whenever used.
A common embedded Rust pattern is to use `Mutex` + `RefCell` for safe access.
`Mutex` is implemented in `cortex-m` crate and requires a critical section
(interrupts disabled), making the access safe/atomic on a single-core MCU.

```rust
static KERNEL: Mutex<RefCell<Option<Kernel>>> = Mutex::new(RefCell::new(None));

// Startup: Initialize kernel
free(|cs| {
    *KERNEL.borrow(cs).borrow_mut() = Some(Kernel::new());
});

// Later: Use kernel
free(|cs| {
    let kernel = KERNEL.borrow(cs).borrow_mut();
});
```

This works because `Mutex` implements the `Sync` trait. However, this approach
has two issues with respect to the current design of `rucos-cortex-m`.

1. Interrupts are disabled a lot, which could cause real-time events to be missed.
2. Disabling interrupts is not necessary in `init()` and not possible in `start()`.

#### Solution Ideas

- Move `KERNEL` to the application
    - Gives flexibility for memory placement
    - `init()` and `start()` can be passed a mutable reference
- Update kernel APIs to use `svc` instruction
    - Acts as a function call into the kernel
    - Does not require accessing the singleton
- Make two versions of `create()`: Before and after kernel start
    - Before kernel start: Pass in a mutable reference to kernel
    - After kernel start: Using `svc` instruction, like other kernel APIs
- Interrupts (SysTick, PendSV, SVCall):
    - Likely need to bind pointer to `KERNEL` in `start()` for ISRs
    - Initial implementation can disable interrupts (`Mutex<RefCell<T>>` pattern)
    - Then need to think more about atomics and other primitives
- Could `Kernel` struct be lighterweight?
    - Each task could have a reference to it's own "TCB" (`Task` struct)
    - Does a linked-list implementation avoid needing a `Vec`?
    - Kernel could use `Atomic` data types for other state (`core::sync::atomic`)
