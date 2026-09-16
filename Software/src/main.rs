#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{
    clock::CpuClock, timer::timg::TimerGroup, spi::{self, master::Spi}, time::Rate,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{SdCard, VolumeManager};

pub mod rfid_module;
pub mod rfid_task;
pub mod time_source;
pub mod sd_card_task;

use esp_println::println;
use log::error;

use crate::{rfid_task::{RFID_CHANNEL, RfidInitData, rfid_task}, time_source::DummyTimeSource};


#[panic_handler]
fn panic(panic_info: &core::panic::PanicInfo) -> ! {
    error!("{}", panic_info);
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

pub const BAUD_RATE: u32 = 38400;

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o log -o vscode

    esp_println::logger::init_logger_from_env();

    println!("Booting up...");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: esp_hal::peripherals::Peripherals = esp_hal::init(config);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let sw_ints =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    let rfid_data = RfidInitData {
        baud_rate: BAUD_RATE,
        uart: peripherals.UART0,
        rx: peripherals.GPIO5,
        tx: peripherals.GPIO6,
    };

    // SD card spi bus
    let spi_bus = 
            Spi::new(peripherals.SPI2, spi::master::Config::default()
            .with_frequency(Rate::from_mhz(1))
            .with_mode(esp_hal::spi::Mode::_0)).unwrap()
            .with_sck(peripherals.GPIO48)
            .with_mosi(peripherals.GPIO38)
            .with_miso(peripherals.GPIO47);

    let cs = esp_hal::gpio::Output::new(peripherals.GPIO21, esp_hal::gpio::Level::High, esp_hal::gpio::OutputConfig::default());

    let spi_dev = ExclusiveDevice::new(spi_bus, cs, embassy_time::Delay).unwrap();

    let sd_card = SdCard::new(spi_dev, embassy_time::Delay);

    let volume_manager = VolumeManager::new(sd_card, DummyTimeSource);

    spawner.spawn(rfid_task(rfid_data, RFID_CHANNEL.sender()).unwrap());

    loop {
        embassy_time::Timer::after_millis(500).await;
    }
}

