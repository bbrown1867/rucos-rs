//! A port of the RuCOS kernel to ARM Cortex-M

#![no_std]

use core::arch::asm;
use core::ptr::write_volatile;
use core::sync::atomic::Ordering;
use cortex_m::peripheral::{scb, syst::SystClkSource, SCB, SYST};
use rucos::kernel;

// Re-export to allow the end user to depend on just one crate
pub use rucos::task::Task;

fn init_task_stack(stack: &[u8], entry: fn(u32) -> !, arg: Option<u32>) -> u32 {
    let mut stack_ptr = stack.as_ptr() as u32 + stack.len() as u32;
    let arg = arg.unwrap_or(0);

    // Align the stack
    stack_ptr &= 0xFFFF_FFF8;

    let register_values = [
        0x0100_0000,                   // xPSR
        entry as *const () as u32,     // PC
        task_exit as *const () as u32, // R14 (LR)
        0x1212_1212,                   // R12
        0x0303_0303,                   // R3
        0x0202_0202,                   // R2
        0x0101_0101,                   // R1
        arg,                           // R0
        0xFFFF_FFFD,                   // R14 (EXC_RETURN)
        0x1111_1111,                   // R11
        0x1010_1010,                   // R10
        0x0909_0909,                   // R9
        0x0808_0808,                   // R8
        0x0707_0707,                   // R7
        0x0606_0606,                   // R6
        0x0505_0505,                   // R5
        0x0404_0404,                   // R4
    ];

    for register_value in register_values {
        stack_ptr -= 4;
        // Safety: We assume the provided stack area is valid and unused
        unsafe { write_volatile(stack_ptr as *mut u32, register_value) };
    }

    stack_ptr
}

/// Initialize the kernel and create the idle task
///
/// # Arguments
///
/// * `idle_stack`: Idle task stack
/// * `user_idle_task`: Optional idle task function
///
/// # Note
///
/// The idle task is the lowest priority task and is always ready to run, it must not block or call
/// any kernel APIs (e.g. [`sleep`]).
pub fn init(idle_stack: &[u8], user_idle_task: Option<fn(u32) -> !>) {
    let idle_stack_ptr = init_task_stack(idle_stack, user_idle_task.unwrap_or(idle_task), None);
    kernel::init(idle_stack_ptr);
}

/// Start the kernel
///
/// # Arguments
///
/// * `scb`: System control block (from `cortex-m` crate)
/// * `systick`: System tick  (from `cortex-m` crate)
/// * `clock_freq_hz`: Core clock frequency in hertz
/// * `tick_rate_hz`: System tick rate in hertz
///
/// # Note
///
/// Does not return: Program execution continues from tasks or interrupt handlers after calling
/// this API.
pub fn start(scb: &mut SCB, systick: &mut SYST, clock_freq_hz: u32, tick_rate_hz: u32) -> ! {
    let first_task = rucos::kernel::get_current_task().unwrap();
    let first_task_stack_ptr = first_task.stack_ptr.load(Ordering::Relaxed);

    systick.set_reload((clock_freq_hz / tick_rate_hz) - 1);
    systick.clear_current();
    systick.set_clock_source(SystClkSource::Core);
    systick.enable_interrupt();
    systick.enable_counter();

    // Safety: Should only be called once from main() before multi-tasking starts
    unsafe {
        // Context switch should only happen once all interrupts have been serviced
        scb.set_priority(scb::SystemHandler::PendSV, 0xFF);

        asm!(
            "cpsid  i",                    // Disable interrupts
            "mov    r0, {tmp}",            // Get first task stack pointer
            "msr    psp, r0",              // Write PSP
            "mrs    r1, control",          // Read CONTROL
            "orr    r1, r1, #2",           // Set SP = PSP
            "bic    r1, r1, #4",           // Clear FPCA (reset FPU)
            "msr    control, r1",          // Write CONTROL
            "isb",                         // Sync instructions
            "ldmia  sp!, {{r4-r11, r14}}", // Restore R4 - R11, LR
            "ldmia  sp!, {{r0-r3}}",       // Restore R0 - R3
            "ldmia  sp!, {{r12, r14}}",    // Load R12 and LR
            "ldmia  sp!, {{r1, r2}}",      // Load PC and discard xPSR
            "cpsie  i",                    // Enable interrupts
            "bx     r1",                   // Branch to first task
            tmp = in(reg) first_task_stack_ptr,
            options(noreturn),
        )
    };
}

/// Create a task
///
/// # Arguments
///
/// * `task`: Task control block
/// * `stack`: Task stack memory
/// * `entry`: Task function
/// * `arg`: An optional argument to pass to `entry`
pub fn create(task: &'static Task, stack: &[u8], entry: fn(u32) -> !, arg: Option<u32>) {
    let task_stack_ptr = init_task_stack(stack, entry, arg);
    kernel::create(task, task_stack_ptr);
}

/// Delete a task
///
/// # Arguments
///
/// * `id`: Task to delete
///
/// # Note
///
/// A context switch may occur after calling this API.
pub fn delete(id: usize) {
    if kernel::delete(id) {
        SCB::set_pendsv();
    }
}

/// Get the ID of the current task
///
/// # Returns
///
/// ID of the current task
pub fn get_current_task() -> usize {
    kernel::get_current_task().unwrap().id
}

/// Get the current value of the kernel tick
///
/// # Returns
///
/// Current value of the kernel tick
///
/// # Note
///
/// Ticks correspond to system time based on `tick_rate_hz` passed to [`start`].
pub fn get_current_tick() -> u32 {
    kernel::get_current_tick()
}

/// Sleep the current task
///
/// # Arguments
///
/// * `delay`: Number of ticks to sleep
///
/// # Note
///
/// Ticks correspond to system time based on `tick_rate_hz` passed to [`start`].
pub fn sleep(delay: u32) {
    kernel::sleep(delay);
    SCB::set_pendsv();
}

/// Suspend a task
///
/// # Arguments
///
/// * `id`: Task to suspend
///
/// # Note
///
/// A context switch may occur after calling this API.
pub fn suspend(id: usize) {
    if kernel::suspend(id) {
        SCB::set_pendsv();
    }
}

/// Resume a task
///
/// # Arguments
///
/// * `id`: Task to resume
pub fn resume(id: usize) {
    kernel::resume(id);
}

/// SysTick interrupt handler: System tick update
#[no_mangle]
extern "C" fn SysTick() {
    if kernel::update_tick(1) {
        SCB::set_pendsv();
    }
}

/// PendSV interrupt handler: Context switch implementation
///
/// # Safety
///
/// Runs with interrupts disabled on a single-core microcontroller.
#[no_mangle]
#[naked_function::naked]
unsafe extern "C" fn PendSV() {
    // TODO: Should work on microcontrollers without floating point hardware
    asm!(
        ".fpu fpv5-d16",                  // Enable FPU instructions
        "cpsid     i",                    // Disable interrupts
        "mrs       r0, psp",              // Read PSP
        "mov       r1, lr",               // Save LR
        "tst       r14, #0x10",           // Check if FPU is being used
        "it        eq",                   // ...
        "vstmdbeq  r0!, {{s16-s31}}",     // Push the FPU registers
        "stmdb     r0!, {{r4-r11, r14}}", // Push the CPU registers
        "push      {{r1}}",               // Push LR
        "bl        context_switch",       // context_switch(R0) -> R0
        "pop       {{r1}}",               // Pop LR
        "ldmia     r0!, {{r4-r11, r14}}", // Pop the CPU registers
        "tst       r14, #0x10",           // Check if FPU is being used
        "it        eq",                   // ...
        "vldmiaeq  r0!, {{s16-s31}}",     // Pop the FPU registers
        "msr       psp, r0",              // Write PSP
        "cpsie     i",                    // Enable interrupts
        "bx        r1",                   // Branch to next task
    );
}

#[no_mangle]
fn context_switch(curr_task_stack_ptr: u32) -> u32 {
    kernel::run_scheduler(curr_task_stack_ptr)
}

fn task_exit() {
    loop {}
}

fn idle_task(_: u32) -> ! {
    loop {}
}
