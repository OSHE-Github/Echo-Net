use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Sender};
use embassy_time::Timer;
use embedded_sdmmc::Timestamp;
use esp_println::println;

use crate::{
    cross_task_data::get_current_time,
    rfid_task::{EPC, RFIDTag},
    ui::ui_task::DeviceButton,
};

#[embassy_executor::task]
pub async fn battery_life_task(
    sender: Sender<'static, CriticalSectionRawMutex, DeviceButton, 16>,
    rfid_sender: Sender<'static, CriticalSectionRawMutex, RFIDTag, 16>,
) {
    // Used to track if we are on loop 4 or not
    let mut loop_index = 0;
    loop {
        // 15 minutes
        Timer::after_secs(15).await;
        // Write to SD
        let mut epc = heapless::Vec::<u8, 12>::new();
        for _ in 0..12 {
            let _ = epc.push(0);
        }

        let tag = RFIDTag {
            rssi: 0,
            freq: 0,
            time_stamp: get_current_time(),
            epc: EPC::new(epc),
        };
        rfid_sender.send(tag).await;

        loop_index += 1;

        if loop_index == 4 {
            loop_index = 0;
            // Scroll through menus
            for _ in 0..5 {
                Timer::after_millis(100).await;
                let _ = sender.send(DeviceButton::MenuButton).await;
            }
        }
    }
}
