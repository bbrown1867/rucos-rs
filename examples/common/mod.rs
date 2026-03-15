use defmt_rtt as _;
use panic_probe as _;
use rucos;
use stm32f7xx_hal::rcc::Clocks;
use stm32f7xx_hal::{pac, prelude::*};

pub const IDLE_STACK_SIZE: usize = 256;
pub const TASK_STACK_SIZE: usize = 512;

pub const TASK0_ID: usize = 0;
pub const TASK1_ID: usize = 1;
pub const TASK0_PRIO: u8 = 10;
pub const TASK1_PRIO: u8 = 11;

defmt::timestamp!("{=u32:us}", rucos::get_current_tick());

pub struct KernelResources {
    pub clocks: Clocks,
}

pub fn setup() -> KernelResources {
    let pac_periph = pac::Peripherals::take().unwrap();
    let rcc = pac_periph.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(16_000_000.Hz()).freeze();
    KernelResources { clocks }
}
