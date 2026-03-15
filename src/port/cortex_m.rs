//! RuCOS ARM Cortex-M Port

use core::arch::asm;
use core::ptr::write_volatile;
use cortex_m::peripheral::scb::SystemHandler;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SCB;
use cortex_m::Peripherals;

use crate::on_system_tick;

const TICK_RATE_HZ: u32 = 1000;

/// Port specific task stack initialization
///
/// # Arguments
///
/// * `stack`: Task stack memory
/// * `entry`: Task function
/// * `arg`: An optional argument to pass to `entry`
///
/// # Returns
///
/// Task stack pointer
pub fn port_init_stack(stack: &mut [u8], entry: fn(u32) -> !, arg: Option<u32>) -> u32 {
    let mut stack_ptr = stack.as_mut_ptr() as u32 + stack.len() as u32;
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

/// Port specific hook to start the kernel
///
/// # Arguments
///
/// * `first_task_stack_ptr`: Stack pointer of first task
/// * `clock_freq_hz`: Core clock frequency in hertz
pub fn port_start(first_task_stack_ptr: u32, clock_freq_hz: u32) -> ! {
    // Safety: The kernel is the exclusive owner of SysTick and SCB
    let cm_periph = unsafe { Peripherals::steal() };
    let mut systick = cm_periph.SYST;
    let mut scb = cm_periph.SCB;

    systick.set_reload((clock_freq_hz / (TICK_RATE_HZ as u32)) - 1);
    systick.clear_current();
    systick.set_clock_source(SystClkSource::Core);
    systick.enable_interrupt();
    systick.enable_counter();

    // Safety: Only called from main task before we start multi-tasking
    unsafe {
        // Context switch should only happen once all interrupts have been serviced
        scb.set_priority(SystemHandler::PendSV, 0xFF);

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

/// Port specific hook to triger a context switch
pub fn port_switch_context() {
    SCB::set_pendsv();
}

/// SysTick ISR: System tick update
#[no_mangle]
pub extern "C" fn SysTick() {
    on_system_tick();
}

/// PendSV ISR: Context switch implementation
///
/// # Safety
///
/// Runs with interrupts disabled on a single-core microcontroller
#[no_mangle]
#[naked_function::naked]
pub unsafe extern "C" fn PendSV() {
    // TODO: Gate use of FPU instructions with a feature flag
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
        "bl        on_context_switch",    // on_context_switch(R0) -> R0
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

/// Tasks should not exit
fn task_exit() {
    loop {}
}
