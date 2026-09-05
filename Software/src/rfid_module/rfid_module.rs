/*

This code is based in part on the SparkFun Simultaneous RFID Tag Reader
library by Nathan Seidle / SparkFun Electronics.

Copyright (c) 2016 SparkFun Electronics

Licensed under the MIT License.

*/

use esp_hal::uart::Uart;

use crate::rfid_module::rfid_constants::MAX_MSG_SIZE;

pub struct RFID {
    pub msg: [u8; MAX_MSG_SIZE],
    msg_head: usize,
    print_debug: bool,
    // Temperary blocking
    uart: Uart<'static, esp_hal::Blocking>,
    debug_uart: Option<Uart<'static, esp_hal::Blocking>>,
}


impl RFID {
    pub fn new(uart: Uart<'static, esp_hal::Blocking>, debug_uart: Uart<'static, esp_hal::Blocking>) -> RFID {
        Self {
            msg: [0; MAX_MSG_SIZE],
            msg_head: 0,
            print_debug: false,
            uart,
            debug_uart: None,
        }
    }

    pub fn enable_debugging(&mut self, debug_uart: Uart<'static, esp_hal::Blocking>) {
        self.debug_uart = Some(debug_uart);
        self.print_debug = true;
    }

    pub fn disable_debugging(&mut self) {
        self.print_debug = false;
    } 

    pub fn set_baudrate(baudrate: u64) {
       // let size = 
    }

    fn send_message(&mut self, opcode: u8, data: &[u8], size: u8, time_out: u16, wait_for_response: bool) {
        self.msg[1] = size;
        self.msg[2] = opcode;

        for i in 0..size as usize {
            if let Some(data) = data.get(i) {
                self.msg[i + 3] = *data;
            }
        }
    }

    fn send_command(&mut self, time_out: u16, wait_for_response: bool) {
        self.msg[0] = 0xFF;
        let message_length = self.msg[1];
        let opcode = self.msg[2];
        
    }
}