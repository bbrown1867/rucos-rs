//! Two periodic tasks, with Task 1 running twice as often as Task 0. The tasks
//! share one "template" for code, with an argument to parametrize them.

#![no_std]
#![no_main]

mod common;

use defmt::info;
use rucos_cortex_m as rucos;

static TASK0: rucos::Task = rucos::Task::new(common::TASK0_ID, common::TASK0_PRIO);
static TASK1: rucos::Task = rucos::Task::new(common::TASK1_ID, common::TASK1_PRIO);

fn task_template(delay_sec: u32) -> ! {
    loop {
        info!("Hello from Task {}", rucos::get_current_task());
        rucos::sleep(delay_sec * common::TICK_RATE_HZ);
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let mut resources = common::setup();

    let idle_stack: [u8; common::IDLE_STACK_SIZE] = [0; common::IDLE_STACK_SIZE];
    rucos::init(&idle_stack, None);

    info!("Creating Task 0");
    let task0_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(&TASK0, &task0_stack, task_template, Some(2));

    info!("Creating Task 1");
    let task1_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(&TASK1, &task1_stack, task_template, Some(1));

    info!("Starting");
    rucos::start(
        &mut resources.scb,
        &mut resources.systick,
        resources.clocks.hclk().to_Hz(),
        common::TICK_RATE_HZ,
    );
}
