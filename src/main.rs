#![no_std]
#![no_main]

extern crate alloc;

use esp_hal::Config;
use esp_hal::main;

use esp_backtrace as _;

use esp_hal::init;

esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let _peripherals = init(Config::default());
    esp_alloc::heap_allocator!(size: 128 * 1024);

    loop {}
}
