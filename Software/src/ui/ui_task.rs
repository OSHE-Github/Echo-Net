use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::{Channel, Receiver}};
use embassy_time::Timer;
use embedded_graphics::{
    Drawable,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    text::Text,
};
use esp_hal::i2c::master::I2c;
use esp_println::println;
use ssd1306::{
    Ssd1306Async, mode::DisplayConfigAsync, prelude::I2CInterface, size::DisplaySize128x64,
};

// Contains data read by the RFID library
pub static UI_CHANNEL: Channel<CriticalSectionRawMutex, &'static str, 16> = Channel::new();

#[embassy_executor::task]
pub async fn ui_driver_task(i2c: I2c<'static, esp_hal::Async>, 
    message_reciever: Receiver<'static, CriticalSectionRawMutex, &'static str, 16>,) {
    let interface = I2CInterface::new(i2c, 0x3C, 0x40);

    let mut display = Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        ssd1306::rotation::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    // Initialize OLED controller
    display.init().await.unwrap();
    println!("UI initialized");

    let text_style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);

    // Write text
    Text::new("Hello world", Point::new(0, 16), text_style)
        .draw(&mut display)
        .unwrap();

    // Send framebuffer to the display
    display.flush().await.unwrap();

    loop {
        let message = message_reciever.receive().await;
        
        display.clear_buffer();
        
        // Write text
        Text::new(message, Point::new(0, 16), text_style)
            .draw(&mut display)
            .unwrap();

        // Send framebuffer to the display
        display.flush().await.unwrap();

    }
}
