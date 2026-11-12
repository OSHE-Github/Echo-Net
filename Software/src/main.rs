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
    clock::CpuClock,
    gpio::{InputConfig, Pull},
    i2c::master::I2c,
    spi::{self, master::Spi},
    time::Rate,
    timer::timg::TimerGroup,
};

// Folders
pub mod rfid_module;
pub mod ui;
// Files
pub mod cross_task_data;
pub mod rfid_task;
pub mod sd_card_task;
pub mod time_source;

use esp_println::println;
use log::error;

use crate::{
    cross_task_data::set_power_level, rfid_task::{RFID_CHANNEL, RfidInitData}, sd_card_task::sd_card_task, time_source::DeviceTimeSource, ui::ui_task::{DeviceButton, UI_CHANNEL, ui_driver_task},
};

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
    esp_println::logger::init_logger_from_env();

    println!("Booting up...");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals: esp_hal::peripherals::Peripherals = esp_hal::init(config);
    // We are using PSRAM for the RFID storage so that we can store many many tags
    esp_alloc::psram_allocator!(peripherals.PSRAM, esp_hal::psram);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let sw_ints =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);

    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    // Buttons
    let mut button_s1 = esp_hal::gpio::Input::new(
        peripherals.GPIO7,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut button_s2 = esp_hal::gpio::Input::new(
        peripherals.GPIO8,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut button_s3 = esp_hal::gpio::Input::new(
        peripherals.GPIO9,
        InputConfig::default().with_pull(Pull::Up),
    );
    let mut button_s4 = esp_hal::gpio::Input::new(
        peripherals.GPIO10,
        InputConfig::default().with_pull(Pull::Up),
    );

    let rfid_data = RfidInitData {
        baud_rate: BAUD_RATE,
        uart: peripherals.UART0,
        rx: peripherals.GPIO5,
        tx: peripherals.GPIO6,
    };

    // SD card spi bus
    let mut spi_bus = Spi::new(
        peripherals.SPI2,
        spi::master::Config::default()
            .with_frequency(Rate::from_khz(300))
            .with_mode(esp_hal::spi::Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO48)
    .with_mosi(peripherals.GPIO38)
    .with_miso(peripherals.GPIO47);

    let i2c_line: I2c<'static, esp_hal::Async> =
        I2c::new(peripherals.I2C0, esp_hal::i2c::master::Config::default())
            .unwrap()
            .with_sda(peripherals.GPIO11)
            .with_scl(peripherals.GPIO12)
            .into_async();

    let mut cs = esp_hal::gpio::Output::new(
        peripherals.GPIO21,
        esp_hal::gpio::Level::High,
        esp_hal::gpio::OutputConfig::default(),
    );

    cs.set_high();

    // Create an array of 10 dummy bytes (all 1s)
    // Otherwise, the SD card won't write properly
    let dummy_bytes: [u8; 10] = [0xFF; 10];
    spi_bus.write(&dummy_bytes).unwrap();

    let spi_dev = ExclusiveDevice::new(spi_bus, cs, embassy_time::Delay).unwrap();
    let sd_card = SdCard::new(spi_dev, embassy_time::Delay);

    let volume_manager = VolumeManager::new(sd_card, cross_task_data::SharedDeviceTimeSource);
    set_power_level(1000);
    // spawner.spawn(rfid_task(rfid_data, RFID_CHANNEL.sender()).unwrap());
    spawner.spawn(sd_card_task(volume_manager, RFID_CHANNEL.receiver()).unwrap());
    spawner.spawn(ui_driver_task(i2c_line, UI_CHANNEL.receiver()).unwrap());

    let sender = UI_CHANNEL.sender();

    loop {
        let button_option = match embassy_futures::select::select4(
            button_s1.wait_for_any_edge(),
            button_s2.wait_for_falling_edge(),
            button_s3.wait_for_falling_edge(),
            button_s4.wait_for_falling_edge(),
        )
        .await
        {
            embassy_futures::select::Either4::First(_) => {
                let button_state = button_s1.level();
                embassy_time::Timer::after_millis(5).await;
                // Make sure it's not bouncing
                if button_s1.level() == button_state {
                    Some(DeviceButton::TriggerButton)
                } else {
                    None
                }
            },
            embassy_futures::select::Either4::Second(_) => {
                embassy_time::Timer::after_millis(5).await;
                if button_s2.is_low() {
                    Some(DeviceButton::MenuButton)
                } else {
                    None
                }
            },
            embassy_futures::select::Either4::Third(_) => {
                embassy_time::Timer::after_millis(5).await;
                if button_s3.is_low() {
                    Some(DeviceButton::PlusButton)
                } else {
                    None
                }
            },
            embassy_futures::select::Either4::Fourth(_) => {
                embassy_time::Timer::after_millis(5).await;
                if button_s4.is_low() {
                    Some(DeviceButton::LockButton)
                } else {
                    None
                }
            },
        };

        if let Some(button) = button_option {
            let _ = sender.try_send(button);
        }

        // Debounce interval
        embassy_time::Timer::after_millis(50).await;
    }
}
