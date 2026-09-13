use embassy_time::Timer;
use esp_hal::uart::{Config, Uart};
use esp_println::println;

use crate::rfid_module::{
    rfid_constants::{DEFAULT_BAUD_RATE, Region, ResponseError},
    rfid_module::RFID,
};

pub struct RfidData {
    pub baud_rate: u32,
    pub uart: esp_hal::peripherals::UART0<'static>,
    pub rx: esp_hal::peripherals::GPIO1<'static>,
    pub tx: esp_hal::peripherals::GPIO2<'static>,
}

#[embassy_executor::task]
pub async fn rfid_task(rfid_data: RfidData) {
    let setup_result = setup_rfid_module(
        rfid_data.baud_rate,
        rfid_data.uart,
        rfid_data.rx,
        rfid_data.tx,
    )
    .await;

    let Ok(mut rfid) = setup_result else {
        panic!("Module failed to respond, please check wiring.");
    };

    rfid.set_region(Region::NorthAmerica);

    // 5.00 dBm. Higher values may caues USB port to brown out
    rfid.set_read_power(2700);
    // Max Read TX Power is 27.00 dBm and may cause temperature-limit throttling

    println!("Modual connected! Constant scanning started.");

    rfid.start_reading();

    loop {
        if rfid.check() {
            let response_type = rfid.parse_response();

            match response_type {
                ResponseError::ResponseIsKeepAlive => {
                    println!("Scanning");
                }
                ResponseError::ResposneIsTagFound => {
                    // If we have a full record we can pull out the fun bits

                    /*
                    The code below is unused because we don't save to the SD card yet
                     */

                    // Get the RSSI for this tag read
                    #[allow(unused)]
                    let rssi = rfid.get_tag_rssi();
                    // Get the frequency this tag was detected at
                    #[allow(unused)]
                    let freq = rfid.get_tag_freq();
                    // Get the time this was read, (ms) since last keep-alive message
                    #[allow(unused)]
                    let time_stamp = rfid.get_tag_time_stamp();
                    // Get the number of bytes of EPC from response
                    #[allow(unused)]
                    let tag_epc_bytes = rfid.get_tag_epc_bytes();

                    todo!("Sd writing");
                }
                ResponseError::ErrorCorruptResponse => {
                    println!("Bad src");
                }
                ResponseError::ResponseIsHighReturnLoss => {
                    println!("High return loss, check antenna!");
                }
                response_error => {
                    println!("Unexpected error: {:?}", response_error);
                }
            }
        }

        Timer::after_millis(500).await;
    }
}

async fn setup_rfid_module(
    baud_rate: u32,
    uart: esp_hal::peripherals::UART0<'static>,
    rx: esp_hal::peripherals::GPIO1<'static>,
    tx: esp_hal::peripherals::GPIO2<'static>,
) -> Result<RFID, ResponseError> {
    let desired_config = Config::default().with_baudrate(baud_rate);

    let rfid_uart: Uart<'_, esp_hal::Blocking> = Uart::new(uart, desired_config)
        .unwrap()
        .with_rx(rx)
        .with_tx(tx);
    let mut rfid = RFID::new(rfid_uart);

    let mut read_buf = [0u8; 1];
    // About 200ms from power on the module will send its firmware version at 115200. We need to ignore this.
    while rfid.uart.read_ready() {
        // We don't care about what's being read
        let _ = rfid.uart.read(&mut read_buf).unwrap();
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
