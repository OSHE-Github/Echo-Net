use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver, Sender},
};
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    text::Text,
};
use esp_hal::i2c::master::I2c;
use esp_println::println;
use heapless::format;
use ssd1306::{
    Ssd1306Async, mode::DisplayConfigAsync, prelude::I2CInterface, size::DisplaySize128x64,
};

use crate::{
    cross_task_data::get_current_power_level, rfid_task::{RFID_SCANNING_ENABLED}, ui::{ui_menu::UIMenu, MENU_LINE_SIZE},
};

pub enum DeviceButton {
    TriggerButton,
    MenuButton,
    PlusButton,
    LockButton,
}

// Contains data read by the RFID library
pub static UI_CHANNEL: Channel<CriticalSectionRawMutex, DeviceButton, 16> = Channel::new();

#[embassy_executor::task]
pub async fn ui_driver_task(
    i2c: I2c<'static, esp_hal::Async>,
    message_reciever: Receiver<'static, CriticalSectionRawMutex, DeviceButton, 16>,
    startup_reciever: Receiver<'static, CriticalSectionRawMutex, bool, 8>,
    epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
) {

    println!("power 1: {}", get_current_power_level());
    let interface = I2CInterface::new(i2c, 0x3C, 0x40);

    let mut display = Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate180,
    )
    .into_buffered_graphics_mode();

    // Initialize OLED controller
    display.init().await.unwrap();
    println!("UI initialized");

    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    let mut ui_menu = UIMenu::new(epc_tag_request_sender);

    let mut currently_reading = false;
    let rfid_reading_sender = RFID_SCANNING_ENABLED.sender();
    display.clear_buffer();

    let [string1, string2, string3] = ui_menu.get_display().await;
    // Write text
    Text::new(
        &string1.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
        Point::new(0, 16),
        text_style,
    )
    .draw(&mut display)
    .unwrap();

    Text::new(
        &string2.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
        Point::new(0, 32),
        text_style,
    )
    .draw(&mut display)
    .unwrap();
    Text::new(
        &string3.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
        Point::new(0, 48),
        text_style,
    )
    .draw(&mut display)
    .unwrap();
    display.flush().await.unwrap();

    startup_reciever.receive().await;
    println!("Startup recieved!");
    ui_menu.load();

    loop {
        display.clear_buffer();

        let [string1, string2, string3] = ui_menu.get_display().await;

        // Write text
        Text::new(
            &string1.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
            Point::new(0, 16),
            text_style,
        )
        .draw(&mut display)
        .unwrap();
        Text::new(
            &string2.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
            Point::new(0, 32),
            text_style,
        )
        .draw(&mut display)
        .unwrap();
        Text::new(
            &string3.unwrap_or(format!(MENU_LINE_SIZE; " ").unwrap()),
            Point::new(0, 48),
            text_style,
        )
        .draw(&mut display)
        .unwrap();

        display.flush().await.unwrap();
        match embassy_futures::select::select(
            message_reciever.receive(),
            embassy_time::Timer::after_millis(100),
        ).await {
            embassy_futures::select::Either::First(button) => {
                match button {
                    DeviceButton::TriggerButton => {
                        currently_reading = !currently_reading;
                        rfid_reading_sender.send(currently_reading);
                        ui_menu.press_trigger_button();
                    }
                    DeviceButton::MenuButton => {
                        ui_menu.press_menu_button().await;
                    }
                    DeviceButton::PlusButton => {
                        ui_menu.press_plus_button();
                    }
                    DeviceButton::LockButton => {
                        ui_menu.press_lock_button().await;
                    }
                }
            }
            embassy_futures::select::Either::Second(_) => {
                // 100 ms elapsed without receiving a button.
            }
        }
    }
}
