use core::fmt::Write;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Receiver};
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{Mode, SdCard, VolumeIdx, VolumeManager};
use esp_hal::{Blocking, gpio::Output, spi::master::Spi};
use esp_println::println;

use crate::{rfid_task::RFIDTag, time_source::DummyTimeSource};

const ECHO_NET_DIRECTORY: &'static str = "EchoNet";

#[embassy_executor::task]
pub async fn sd_card_task(
    volume_manager: VolumeManager<
        SdCard<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>, Delay>,
        DummyTimeSource,
    >,
    rfid_data_reciever: Receiver<'static, CriticalSectionRawMutex, RFIDTag, 8>,
) {
    let volume = volume_manager.open_volume(VolumeIdx(0)).unwrap();

    // If there's no panics, it means the SD card successfully connected
    println!("SD card connected");

    let root_dir = volume_manager
        .open_root_dir(volume.to_raw_volume())
        .unwrap();

    let echo_net_dir = match volume_manager.open_dir(root_dir, ECHO_NET_DIRECTORY) {
        Ok(dir) => dir,
        Err(_) => {
            // If directory can't be opened, create it
            volume_manager
                .make_dir_in_dir(root_dir, ECHO_NET_DIRECTORY)
                .unwrap();
            volume_manager
                .open_dir(root_dir, ECHO_NET_DIRECTORY)
                .unwrap()
        }
    };

    let mut filename = heapless::String::<32>::new();

    for idx in 1..100 {
        filename.clear();
        write!(filename, "log_{:02}.txt", idx).unwrap();

        match volume_manager.open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadOnly) {
            Ok(file) => {
                // It exists, so close it and try the next number.
                volume_manager.close_file(file).unwrap();
            }
            Err(_) => {
                // Doesn't exist, so this is our filename.
                break;
            }
        }
    }

    let file = volume_manager
        .open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadWriteCreate)
        .unwrap();

    volume_manager.close_file(file).unwrap();

    println!("File created");

    loop {
        let rfid_data = rfid_data_reciever.receive().await;
        let formatted_rfid_data = format_rfid_data(&rfid_data);
    }
}

fn format_rfid_data(data: &RFIDTag) -> heapless::String<256> {
    let mut output = heapless::String::new();

    write!(
        output,
        "RSSI: {} Frequency: {} Timestamp: {} EPC[",
        data.rssi, data.freq, data.time_stamp,
    )
    .unwrap();

    for (i, byte) in data.epc_bytes.iter().enumerate() {
        if i > 0 {
            write!(output, " ").unwrap();
        }

        write!(output, "{:02X}", byte).unwrap();
    }

    writeln!(output, "]").unwrap();

    output
}
