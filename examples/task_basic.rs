//! Two periodic tasks, with Task 1 running twice as often as Task 0. The tasks
//! share one "template" for code, with an argument to parametrize them.

#![no_std]
#![no_main]

mod common;

use defmt::info;
use rucos;

static TASK0: rucos::Task = rucos::Task::new(common::TASK0_ID, common::TASK0_PRIO);
static TASK1: rucos::Task = rucos::Task::new(common::TASK1_ID, common::TASK1_PRIO);

fn task_template(delay: u32) -> ! {
    loop {
        info!("Hello from Task {}", rucos::get_current_task().id);
        rucos::sleep(delay);
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let resources = common::setup();

    info!("Initializing kernel");
    let mut idle_stack: [u8; common::IDLE_STACK_SIZE] = [0; common::IDLE_STACK_SIZE];
    rucos::init(&mut idle_stack, None);

    info!("Creating Task 0");
    let mut task0_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(&TASK0, &mut task0_stack, task_template, Some(2000));

    info!("Creating Task 1");
    let mut task1_stack: [u8; common::TASK_STACK_SIZE] = [0; common::TASK_STACK_SIZE];
    rucos::create(&TASK1, &mut task1_stack, task_template, Some(1000));

    info!("Starting");
    rucos::start(resources.clocks.hclk().to_Hz());
}
