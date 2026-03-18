use cortex_m::peripheral::{SCB, SYST};
use defmt_rtt as _;
use panic_probe as _;
use rucos_cortex_m as rucos;
use stm32f7xx_hal::rcc::Clocks;
use stm32f7xx_hal::{pac, prelude::*};

pub const TICK_RATE_HZ: u32 = 1000;

pub const IDLE_STACK_SIZE: usize = 256;
pub const TASK_STACK_SIZE: usize = 1024;

pub const TASK0_ID: usize = 0;
pub const TASK1_ID: usize = 1;
pub const TASK0_PRIO: u8 = 10;
pub const TASK1_PRIO: u8 = 11;

// NOTE: Kernel must be initialized before using defmt macros
defmt::timestamp!("{=u32:ms}", rucos::get_current_tick());

pub struct KernelResources {
    pub scb: SCB,
    pub systick: SYST,
    pub clocks: Clocks,
}

pub fn setup() -> KernelResources {
    let pac_periph = pac::Peripherals::take().unwrap();
    let rcc = pac_periph.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(16_000_000.Hz()).freeze();

    let cm_periph = cortex_m::Peripherals::take().unwrap();
    let systick = cm_periph.SYST;
    let scb = cm_periph.SCB;

    KernelResources {
        scb,
        systick,
        clocks,
    }
}
