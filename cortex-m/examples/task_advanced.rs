//! Two tasks used in the following sequence:
//! - Task 0 suspends itself after 5 seconds
//! - Task 1 resumes Task 0 after 10 seconds
//! - Task 1 deletes Task 0 after 15 seconds
//! - Task 1 deletes itself after 20 seconds

#![no_std]
#![no_main]

mod common;

use defmt::info;
use rucos_cortex_m as rucos;

fn task0(_: u32) -> ! {
    let mut counter = 0;

    loop {
        if counter == 5 {
            info!("Task 0 suspending itself");
            rucos::suspend(None);
        } else {
            info!("Hello from Task {}", rucos::get_current_task());
            rucos::sleep(rucos::TICK_RATE_HZ);
        }

        counter += 1;
    }
}

fn task1(_: u32) -> ! {
    let mut counter = 0;

    loop {
        if counter == 10 {
            info!("Task 1 resuming Task 0");
            rucos::resume(0);
            rucos::sleep(rucos::TICK_RATE_HZ);
        } else if counter == 15 {
            info!("Task 1 deleting Task 0");
            rucos::delete(Some(0));
            rucos::sleep(rucos::TICK_RATE_HZ);
        } else if counter == 20 {
            info!("Task 1 deleting itself");
            rucos::delete(None);
        } else {
            info!("Hello from Task {}", rucos::get_current_task());
            rucos::sleep(rucos::TICK_RATE_HZ);
        }

        counter += 1;
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
    rucos::create(0, 0, &mut task0_stack, task0, None);

    info!("Creating Task 1");
    let mut task1_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(1, 1, &mut task1_stack, task1, None);

    info!("Starting");
    rucos::start(
        &mut resources.scb,
        &mut resources.systick,
        resources.clocks.hclk().to_Hz(),
    );
}
