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
            // If directory can't be opened, create it first
            volume_manager
                .make_dir_in_dir(root_dir, ECHO_NET_DIRECTORY)
                .unwrap();
            volume_manager
                .open_dir(root_dir, ECHO_NET_DIRECTORY)
                .unwrap()
        }
    };

    let mut filename = heapless::String::<32>::new();
    let mut filename_found = false;

    // Not really sure what our max amount of files should be. Previously a u8 was used so the max was 255.
    for idx in 1..1000 {
        filename.clear();
        write!(filename, "log_{:02}.txt", idx).unwrap();

        match volume_manager.open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadOnly) {
            Ok(file) => {
                // It exists, so close it and try the next number.
                volume_manager.close_file(file).unwrap();
            }
            Err(_) => {
                // Doesn't exist, so this is our filename.
                filename_found = true;
                break;
            }
        }
    }

    if !filename_found {
        panic!("SD card full!");
    }

    let file = volume_manager
        .open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadWriteCreate)
        .unwrap();
    // We can close the file now that we have it
    volume_manager.close_file(file).unwrap();

    println!("File created");

    loop {
        let rfid_data = rfid_data_reciever.receive().await;
        let log_entry = format_rfid_data(&rfid_data);
        let file = volume_manager
            .open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadWriteAppend)
            .unwrap()
            .to_file(&volume_manager);
        file.write(log_entry.as_bytes()).unwrap();
        volume_manager.close_file(file.to_raw_file()).unwrap();
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
