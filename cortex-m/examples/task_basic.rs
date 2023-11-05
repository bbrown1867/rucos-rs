//! Two periodic tasks, with Task 1 running twice as often as Task 0. The tasks
//! share one "template" for code, with an argument to parametrize them.

#![no_std]
#![no_main]

mod common;

use defmt::info;
use rucos_cortex_m as rucos;

fn task_template(arg: u32) -> ! {
    let delay: u64 = arg as u64;
    assert!(delay > 0);

    loop {
        info!("Hello from Task {}", rucos::get_current_task());
        rucos::sleep(delay * rucos::TICK_RATE_HZ);
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut resources = common::setup();

    info!("Initializing");
    let mut idle_stack: [u8; common::IDLE_STACK_SIZE] = [0; common::IDLE_STACK_SIZE];
    rucos::init(&mut idle_stack, None);

    info!("Creating Task 0");
    let mut task0_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(0, 0, &mut task0_stack, task_template, Some(2));

    info!("Creating Task 1");
    let mut task1_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(1, 1, &mut task1_stack, task_template, Some(1));

    info!("Starting");
    rucos::start(
        &mut resources.scb,
        &mut resources.systick,
        resources.clocks.hclk().to_Hz(),
    );
}
