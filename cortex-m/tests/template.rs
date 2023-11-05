#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

#[defmt_test::tests]
mod tests {
    use defmt::info;
    use stm32f7xx_hal as _;

    #[test]
    fn task_create_and_sleep() {
        info!("Currently not using defmt-test - this file is kept for future reference");
    }
}
