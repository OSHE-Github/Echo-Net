use core::fmt::Write;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::{Receiver, Sender}};
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::{BlockDevice, Mode, RawDirectory, SdCard, TimeSource, VolumeIdx, VolumeManager};
use esp_hal::{Blocking, gpio::Output, spi::master::Spi};
use esp_println::println;

use crate::{cross_task_data::SharedDeviceTimeSource, rfid_task::{EPC, RFIDTag}};

const ECHO_NET_DIRECTORY: &'static str = "ECHONET";

#[embassy_executor::task]
pub async fn sd_card_task(
    volume_manager: VolumeManager<
        SdCard<ExclusiveDevice<Spi<'static, Blocking>, Output<'static>, Delay>, Delay>,
        SharedDeviceTimeSource,
    >,
    rfid_data_reciever: Receiver<'static, CriticalSectionRawMutex, RFIDTag, 16>,
    epc_tag_request_reciever: Receiver<'static, CriticalSectionRawMutex, usize, 8>,
    epc_tag_sender: Sender<'static, CriticalSectionRawMutex, EPC, 8>,
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
        match embassy_futures::select::select(rfid_data_reciever.receive(), epc_tag_request_reciever.receive()).await {
            embassy_futures::select::Either::First(rfid_data) => {
                let log_entry = format_rfid_data(&rfid_data);
                let file = volume_manager
                    .open_file_in_dir(echo_net_dir, filename.as_str(), Mode::ReadWriteAppend)
                    .unwrap()
                    .to_file(&volume_manager);
                file.write(log_entry.as_bytes()).unwrap();
                volume_manager.close_file(file.to_raw_file()).unwrap();
            },
            embassy_futures::select::Either::Second(tag_index) => {
                let epc_result = get_stored_rfid(&volume_manager, echo_net_dir, filename.as_str(), tag_index);
                if let Ok(Some(epc)) = epc_result {
                    epc_tag_sender.send(epc).await;
                }
            }
        }
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

    for (i, byte) in data.epc.bytes.iter().enumerate() {
        if i > 0 {
            write!(output, " ").unwrap();
        }

        write!(output, "{:02X}", byte).unwrap();
    }

    writeln!(output, "]").unwrap();

    output
}

pub fn format_epc(epc: &EPC) -> heapless::String<16> {
    let mut output = heapless::String::new();

    for (i, byte) in epc.bytes.iter().enumerate() {
        if i > 0 {
            write!(output, " ").unwrap();
        }

        write!(output, "{:02X}", byte).unwrap();
    }

    output
}



pub fn get_stored_rfid<D, T>(
    volume_manager: &VolumeManager<D, T>,
    directory: RawDirectory,
    filename: &str,
    tag_index: usize,
) -> Result<Option<EPC>, ()>
where
    D: BlockDevice,
    T: TimeSource,
{
    let file = volume_manager.open_file_in_dir(
        directory,
        filename,
        Mode::ReadOnly,
    ).unwrap();

    // Read the file in small chunks.
    let result = (|| {
        let mut buffer = [0u8; 64];
        let mut line_index = 0usize;

        let mut marker_index = 0usize;
        let marker = b"EPC[";

        let mut found_marker = false;
        let mut epc_bytes = heapless::Vec::<u8, 12>::new();
        let mut high_nibble: Option<u8> = None;

        loop {
            let bytes_read = volume_manager.read(file, &mut buffer).unwrap();

            if bytes_read == 0 {
                return Ok(None);
            }

            for &byte in &buffer[..bytes_read] {
                // Only process the requested line.
                if line_index > tag_index {
                    return Ok(None);
                }

                // Move to the next line.
                if byte == b'\n' {
                    if line_index == tag_index {
                        return Ok(None);
                    }

                    line_index += 1;
                    marker_index = 0;
                    found_marker = false;
                    high_nibble = None;
                    epc_bytes.clear();

                    continue;
                }

                // Ignore lines before the requested index.
                if line_index != tag_index {
                    continue;
                }

                // Find "EPC[".
                if !found_marker {
                    if byte == marker[marker_index] {
                        marker_index += 1;

                        if marker_index == marker.len() {
                            found_marker = true;
                        }
                    } else {
                        marker_index = if byte == marker[0] {
                            1
                        } else {
                            0
                        };
                    }

                    continue;
                }

                // Found the closing bracket.
                if byte == b']' {
                    if high_nibble.is_some() || epc_bytes.len() != 12 {
                        return Ok(None);
                    }

                    return Ok(Some(EPC::new(epc_bytes)));
                }

                // Ignore spaces and carriage returns.
                if byte == b' ' || byte == b'\r' || byte == b'\t' {
                    continue;
                }

                // Convert hexadecimal character into a nibble.
                let nibble = match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    b'A'..=b'F' => byte - b'A' + 10,
                    _ => return Ok(None),
                };

                if let Some(high) = high_nibble.take() {
                    let value = (high << 4) | nibble;

                    if epc_bytes.push(value).is_err() {
                        return Ok(None);
                    }
                } else {
                    high_nibble = Some(nibble);
                }
            }
        }
    })();

    // Always close the file, including when reading fails.
    let _ = volume_manager.close_file(file).unwrap();

    let epc = result?;

    Ok(epc)
}
