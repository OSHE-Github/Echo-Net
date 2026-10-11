use allocator_api2::vec::Vec;
use core::fmt::Write;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Sender},
};
use embassy_time::Timer;
use esp_alloc::ExternalMemory;
use esp_hal::uart::{Config, Uart};
use esp_println::println;
use hashbrown::HashSet;
use log::{info, warn};

use crate::rfid_module::{
    rfid_constants::{DEFAULT_BAUD_RATE, Region, ResponseError},
    rfid_module::RFID,
};

pub struct RfidInitData {
    pub baud_rate: u32,
    pub uart: esp_hal::peripherals::UART0<'static>,
    pub rx: esp_hal::peripherals::GPIO5<'static>,
    pub tx: esp_hal::peripherals::GPIO6<'static>,
}

pub struct RFIDTag {
    pub rssi: i8,
    pub freq: u32,
    pub time_stamp: u16,
    pub epc: EPC,
}

// We can track up to 100,000 different bats in one session
const MAX_TAGS: usize = 100_000;

// Contains data read by the RFID library
pub static RFID_CHANNEL: Channel<CriticalSectionRawMutex, RFIDTag, 16> = Channel::new();

#[derive(Clone, PartialEq, Eq)]
pub struct EPC {
    // EPC tags can technically be up to 62 bytes, ours are 12 though.
    // Keep an eye on RAM usage, possibly change design.
    // 12 bytes * 10,000 possible tags = 120,000 bytes / 1024 = 117.2 KB
    // Currently exists in SRAM, using 117.2 KB / 512 KB = 23.8% of SRAM. May switch to external RAM.
    pub bytes: heapless::Vec<u8, 12>,
}

impl EPC {
    pub fn new(bytes: heapless::Vec<u8, 12>) -> Self {
        Self { bytes }
    }
}

#[embassy_executor::task]
pub async fn rfid_task(
    rfid_data: RfidInitData,
    rfid_sender: Sender<'static, CriticalSectionRawMutex, RFIDTag, 16>,
) {
    // PSRAM storage for the EPC tags. Stored in a has for faster access
    let mut unique_epc_tags = create_tag_storage();

    let setup_result = setup_rfid_module(
        rfid_data.baud_rate,
        rfid_data.uart,
        rfid_data.rx,
        rfid_data.tx,
    )
    .await;

    let mut rfid = match setup_result {
        Ok(rfid) => rfid,
        Err(response_error) => {
            panic!(
                "Module failed to respond, please check wiring {:?}.",
                response_error
            );
        }
    };

    rfid.set_region(Region::NorthAmerica);

    // 5.00 dBm. Higher values may caues USB port to brown out
    rfid.set_read_power(1000);
    // Max Read TX Power is 27.00 dBm and may cause temperature-limit throttling

    println!("Module connected! Constant scanning started.");

    rfid.start_reading();

    loop {
        if rfid.check().await {
            let response_type = rfid.parse_response();

            match response_type {
                ResponseError::ResponseIsKeepAlive => {
                    println!("Scanning");
                }
                ResponseError::ResposneIsTagFound => {
                    // If we have a full record we can pull out the fun bits

                    // Get the RSSI for this tag read
                    let rssi = rfid.get_tag_rssi();
                    // Get the frequency this tag was detected at
                    let freq = rfid.get_tag_freq();
                    // Get the time this was read, (ms) since last keep-alive message
                    let time_stamp = rfid.get_tag_time_stamp();
                    // Get the number of bytes of EPC from response
                    let tag_epc_bytes = rfid.get_tag_epc_bytes();

                    println!(
                        "rssi: {rssi}, freq: {freq}, time stamp: {time_stamp}, tag epc bytes: {tag_epc_bytes}"
                    );
                    let mut output = heapless::String::<128>::new();
                    let mut epc_bytes: heapless::Vec<u8, 12> = heapless::Vec::new();

                    write!(output, " epc[").unwrap();

                    for idx in 0..usize::from(tag_epc_bytes) {
                        write!(output, "{:02X} ", rfid.msg[31 + idx]).unwrap();
                        epc_bytes.push(rfid.msg[31 + idx]).unwrap();
                    }

                    let epc = EPC::new(epc_bytes);

                    write!(output, "]").unwrap();

                    let result = check_if_unique(unique_epc_tags, &epc);
                    match result {
                        Ok(true) => {
                            println!("saving: {}", output);
                        }
                        Ok(false) => {
                            println!("found: {}", output);
                            continue;
                        }
                        Err(_) => {
                            warn!("RFID tag PSRAM is full!");
                            continue;
                        }
                    }

                    let rfid_tag = RFIDTag {
                        rssi,
                        freq,
                        time_stamp,
                        epc,
                    };

                    rfid_sender.send(rfid_tag).await;
                }
                ResponseError::ErrorCorruptResponse => {
                    println!("Bad crc");
                }
                ResponseError::ResponseIsHighReturnLoss => {
                    println!("High return loss, check antenna!");
                }
                response_error => {
                    println!("Unexpected error: {:?}", response_error);
                }
            }
        }
    }
}

async fn setup_rfid_module(
    baud_rate: u32,
    uart: esp_hal::peripherals::UART0<'static>,
    rx: esp_hal::peripherals::GPIO5<'static>,
    tx: esp_hal::peripherals::GPIO6<'static>,
) -> Result<RFID, ResponseError> {
    let desired_config = Config::default().with_baudrate(baud_rate);

    let rfid_uart: Uart<'_, esp_hal::Async> = Uart::new(uart, desired_config)
        .unwrap()
        .with_rx(rx)
        .with_tx(tx)
        .into_async();
    let mut rfid = RFID::new(rfid_uart);

    // Wait for RFID module to boot up
    Timer::after(embassy_time::Duration::from_millis(100)).await;

    let mut read_buf = [0u8; 16];
    // About 200ms from power on the module will send its firmware version at 115200. We need to ignore this.
    while rfid.uart.read_ready() {
        // We don't care about what's being read
        match rfid.uart.read(&mut read_buf) {
            Ok(_data) => {}
            Err(err) => {
                info!("UART read error: {:?}", err);
            }
        }
    }

    rfid.get_version();

    if let Some(response_error) = rfid.get_response_error()
        && response_error == ResponseError::ErrorWrongOpcodeResponse
    {
        rfid.stop_reading();
        println!("Module continuously reading. Asking it to stop...");
        Timer::after(embassy_time::Duration::from_millis(1500)).await;
    } else {
        rfid.uart
            .apply_config(&Config::default().with_baudrate(DEFAULT_BAUD_RATE))
            .unwrap();
        rfid.set_baudrate(u64::from(baud_rate));
        rfid.uart.apply_config(&desired_config).unwrap();
        Timer::after(embassy_time::Duration::from_millis(250)).await;
    }

    // Test the connection
    rfid.get_version();

    match rfid.get_response_error() {
        Some(ResponseError::AllGood) => {
            // Set protocol to GEN2
            rfid.set_tag_protocol(None);
            // Set TX/RX antenna ports to 1
            rfid.set_antenna_port();
            Ok(rfid)
        }
        Some(response_error) => Err(response_error),
        None => Err(ResponseError::ResponseIsUnknown),
    }
}

fn create_tag_storage() -> HashSet<EPC, _, ExternalMemory> {
    let mut tags =
        HashSet::with_capacity_and_hasher_in(MAX_TAGS, Default::default(), ExternalMemory);

    // If we can't store the tags then fail
    tags.try_reserve_exact(MAX_TAGS)
        .expect("Failed to allocate RFID tag storage in PSRAM");

    tags
}

fn check_if_unique(
    tags: &mut HashSet<EPC, hashbrown::DefaultHashBuilder, ExternalMemory>,
    tag: RfidTag,
) -> Result<bool, ()> {
    if tags.contains(&tag) {
        return Ok(false);
    }

    if tags.len() >= MAX_TAGS {
        return Err(());
    }

    Ok(tags.insert(tag))
}
