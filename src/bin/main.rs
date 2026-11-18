#![no_std]
#![no_main]

use core::panic::PanicInfo;

use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};

esp_bootloader_esp_idf::esp_app_desc!();

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    let mut backlight = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    let delay = Delay::new();

    loop {
        backlight.set_high();
        delay.delay_millis(1000);

        backlight.set_low();
        delay.delay_millis(1000);
    }
}
