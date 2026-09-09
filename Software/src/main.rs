#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, VolumeManager};
use esp_hal::{
    clock::CpuClock, main, spi::{self, master::Spi}, time::{Duration, Instant, Rate}, uart::Uart,
};

pub mod rfid_module;
pub mod sd_card;

use log::info;
use log::error;

#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}


// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o log -o vscode
    esp_println::logger::init_logger_from_env();

    todo!("Decide if we actually need max clock speed");
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let mut uart: Uart<'_, esp_hal::Blocking> = Uart::new(peripherals.UART0, Config::default())?
    .with_rx(peripherals.GPIO1)
    .with_tx(peripherals.GPIO2);

    let spi_bus = 
            Spi::new(peripherals.SPI2, spi::master::Config::default()
            .with_frequency(Rate::from_mhz(1))
            .with_mode(esp_hal::spi::Mode::_0)).unwrap()
            .with_sck(todo!("figure out sck"))
            .with_mosi(todo!("figure out mosi"))
            .with_miso(todo!("figure out miso"));

    let spi_dev = ExclusiveDevice::new(spi_bus, todo!("figure out cs"), embassy_time::Delay).unwrap();

    let sd_card = SdCard::new(spi_dev, embassy_time::Delay);

    let volume_manager = VolumeManager::new(sd_card, todo!("figure out time source"));

    loop {
        info!("Hello world!");
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}


