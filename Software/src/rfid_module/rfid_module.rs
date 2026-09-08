use core::num::TryFromIntError;

/*

This code is based in part on the SparkFun Simultaneous RFID Tag Reader
library by Nathan Seidle / SparkFun Electronics.

Copyright (c) 2016 SparkFun Electronics

Licensed under the MIT License.

*/
use esp_hal::{
    time::{Duration, Instant},
    uart::Uart,
};

use crate::rfid_module::rfid_constants::{
    COMMAND_TIME_OUT, CRC_TABLE, MAX_MSG_SIZE,
    ResponseError::{self, AllGood, ResponseSuccess},
    TMR_SR_OPCODE_GET_READER_OPTIONAL_PARAMS, TMR_SR_OPCODE_GET_WRITE_TX_POWER,
    TMR_SR_OPCODE_KILL_TAG, TMR_SR_OPCODE_READ_TAG_DATA, TMR_SR_OPCODE_READ_TAG_ID_MULTIPLE,
    TMR_SR_OPCODE_SET_READ_TX_POWER, TMR_SR_OPCODE_SET_READER_OPTIONAL_PARAMS,
    TMR_SR_OPCODE_SET_WRITE_TX_POWER, TMR_SR_OPCODE_VERSION, TMR_SR_OPCODE_WRITE_TAG_DATA,
};

pub struct RFID {
    pub msg: [u8; MAX_MSG_SIZE],
    msg_head: usize,
    print_debug: bool,
    // Temperary blocking
    uart: Uart<'static, esp_hal::Blocking>,
    debug_uart: Option<Uart<'static, esp_hal::Blocking>>,
    response_error: Option<ResponseError>,
}

impl RFID {
    pub fn new(uart: Uart<'static, esp_hal::Blocking>) -> RFID {
        Self {
            msg: [0; MAX_MSG_SIZE],
            msg_head: 0,
            print_debug: false,
            uart,
            debug_uart: None,
            response_error: None,
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

    // Sends optional parameters to the module
    // See TMR_SR_Configuration in serial_reader_imp.h for a breakdown of options
    pub fn set_read_configuration(&mut self, option1: u8, option2: u8) {
        let data = [1, option1, option2];

        self.send_message(
            TMR_SR_OPCODE_SET_READER_OPTIONAL_PARAMS,
            &data,
            u8::try_from(data.len()).unwrap(),
            COMMAND_TIME_OUT,
            true,
        );
    }

    // Gets optional parameters from the module
    // We know only the blob and are not able to yet identify what each parameter does
    pub fn get_optional_parameters(&mut self, option1: u8, option2: u8) {
        // These are parameters gleaned from inspecting the 'Transport Logs' of the Universal Reader Assistant
        // During setup the software pings different options
        let data = [option1, option2];

        self.send_message(
            TMR_SR_OPCODE_GET_READER_OPTIONAL_PARAMS,
            &data,
            u8::try_from(data.len()).unwrap(),
            COMMAND_TIME_OUT,
            true,
        );
    }

    //Get the version number from the module
    pub fn get_version(&mut self) {
        self.send_message(TMR_SR_OPCODE_VERSION, &[], 0, COMMAND_TIME_OUT, true);
    }

    // Set the read TX power
    // Maximum power is 2700 = 27.00 dBm
    // 1005 = 10.05dBm
    pub fn set_read_power(&mut self, mut power_setting: u16) {
        // Limit to 27d8m
        power_setting.clamp(power_setting, 2700);

        // Copy this setting into a temp data array
        let data = power_setting.to_be_bytes();

        self.send_message(
            TMR_SR_OPCODE_SET_READ_TX_POWER,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );
    }

    // Get the read TX power
    pub fn get_read_power(&mut self) {
        let data = [0x00u8];

        self.send_message(
            TMR_SR_OPCODE_GET_READ_TX_POWER,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );
    }

    // Set the write power
    // Maximum power is 2700 = 27.00 dBm
    // 1005 = 10.05dBm
    pub fn set_write_power(&mut self, power_setting: u16) {
        let data = power_setting.to_be_bytes();

        self.send_message(
            TMR_SR_OPCODE_SET_WRITE_TX_POWER,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );
    }

    // Get the write TX power
    pub fn get_write_power(&mut self) {
        let data = [0x00u8];

        self.send_message(
            TMR_SR_OPCODE_GET_WRITE_TX_POWER,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );
    }

    // Read a single EPC
    // Caller must provide an array for EPC to be stored in
    pub fn read_tag_epc(&mut self, epc: &mut [u8], time_out: u16) -> Result<usize, ResponseError> {
        // User data bank
        let bank = 0x011;
        // Starts at 2
        let address = 0x02;

        self.read_data(bank, address, epc, time_out)
    }

    // This writes a new EPC to the first tag it detects
    // Use with caution. This function doesn't control which tag hears the command.
    pub fn write_tag_epc(&mut self, new_id: &mut [u8], time_out: u16) -> Result<(), ResponseError> {
        todo!("new_id might need to be a &mut [char] or some other data type");
        // EPC memory
        let bank = 0x01;
        // EPC starts at spot 4
        let address = 0x02;

        self.write_data(bank, address, new_id, time_out);
    }

    // This reads the user data area of the tag. 0 to 64 bytes are normally available.
    // Use with caution. The module can't control which tag hears the command.
    pub fn read_user_data(
        &mut self,
        user_data: &mut [u8],
        time_out: u16,
    ) -> Result<usize, ResponseError> {
        // User data bank
        let bank = 0x03;
        // Starts at 0
        let address = 0x00;

        self.read_data(bank, address, user_data, time_out)
    }

    // This writes data to the tag. 0, 4, 16 or 64 bytes may be available.
    // Writes to the first spot 0x00 and fills up as much of the bytes as user provides
    // Use with caution. Function doesn't control which tag hears the command.
    pub fn write_user_data(
        &mut self,
        user_data: &mut [u8],
        time_out: u16,
    ) -> Result<(), ResponseError> {
        // User memory
        let bank = 0x03;
        let address = 0x00;

        self.write_data(bank, address, user_data, time_out)
    }

    // Write the kill password. Should be 4 bytes long
    pub fn write_kill_pw(
        &mut self,
        password: &mut [u8],
        time_out: u16,
    ) -> Result<(), ResponseError> {
        // Passwords bank
        let bank = 0x00;
        // Kill password address
        let address = 0x00;

        self.write_data(bank, address, password, time_out)
    }

    // Read the kill password. Should be 4 bytes long
    pub fn read_kill_rw(
        &mut self,
        password: &mut [u8],
        time_out: u16,
    ) -> Result<usize, ResponseError> {
        // Passwords bank
        let bank = 0x00;
        // Kill password address
        let address = 0x00;

        self.read_data(bank, address, password, time_out)
    }

    // Write the access password. Should be 4 bytes long
    pub fn write_access_pw(
        &mut self,
        password: &mut [u8],
        time_out: u16,
    ) -> Result<(), ResponseError> {
        // Passwords bank
        let bank = 0x00;
        // Access password address
        let address = 0x02;

        self.write_data(bank, address, password, time_out)
    }

    // Read the access password. Should be 4 bytes long
    pub fn read_access_pw(
        &mut self,
        password: &mut [u8],
        time_out: u16,
    ) -> Result<usize, ResponseError> {
        // Passwords bank
        let bank = 0x00;
        // Access password address
        let address = 0x02;

        self.read_data(bank, address, password, time_out)
    }

    // Read the unique TID of the tag. Should be 20 bytes long
    // This is a depricated function left in place in case users still use the readTID command
    // This function is actually reading the UID. To read the TID, including the Chip Vendor
    // change the address to 0x00.
    pub fn read_tid(&mut self, tid: &mut [u8], time_out: u16) -> Result<usize, ResponseError> {
        // Bank for TID
        let bank = 0x02;
        let address = 0x02;

        self.read_data(bank, address, tid, time_out)
    }

    // Read the unique ID of the tag. Can vary from 0 to 20 or more bytes
    pub fn read_uid(&mut self, tid: &mut [u8], time_out: u16) -> Result<usize, ResponseError> {
        // Bank for TID
        let bank = 0x02;
        // UID of the TID starts at 4
        let address = 0x02;

        self.read_data(bank, address, tid, time_out)
    }

    // Writes a data array to a given bank and address
    // Allows for writing of passwords and user data
    pub fn write_data(
        &mut self,
        bank: u8,
        address: u32,
        data_to_record: &mut [u8],
        time_out: u16,
    ) -> Result<(), ResponseError> {
        // Example: FF  0A  24  03  E8  00  00  00  00  00  03  00  EE  58  9D
        // FF 0A 24 = Header, LEN, Opcode
        // 03 E8 = Timeout in ms
        // 00 = Option initialize
        // 00 00 00 00 = Address
        // 03 = Bank
        // 00 EE = Data
        // 58 9D = CRC
        let mut data = [0u8; 8 + data_to_record.len()];

        // Insert timeout
        data[0..2].copy_from_slice(&time_out.to_be_bytes());

        // Option initialize
        data[2] = 0x00;

        // Splice address into array
        data[3..7].copy_from_slice(&address.to_be_bytes());

        // Bank 0 = Passwords
        // Bank 1 = EPC Memory Bank
        // Bank 2 = TID
        // Bank 3 = User Memory
        data[7] = bank;

        data[8..].copy_from_slice(&data_to_record);

        self.send_message(
            TMR_SR_OPCODE_WRITE_TAG_DATA,
            data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );

        if self.response_error == Some(ResponseError::AllGood) {
            let status = u16::from_be_bytes([self.msg[3], self.msg[4]]);

            if status == 0x0000 {
                return Ok(());
            }
        }

        // Else - msg[0] was timeout or other
        Err(ResponseError::ResponseFail)
    }

    // Reads a given bank and address to a data array
    // Allows for writing of passwords and user data
    pub fn read_data(
        &mut self,
        bank: u8,
        address: u32,
        data_read: &mut [u8],
        time_out: u16,
    ) -> Result<usize, ResponseError> {
        // Bank 0
        // response: [00] [08] [28] [00] [00] [10] [00] [00] [EE] [FF] [11] [22] [12] [34] [56] [78]
        // [EE] [FF] [11] [22] = Kill pw
        // [12] [34] [56] [78] = Access pw

        // Bank 1
        // response: [00] [08] [28] [00] [00] [10] [00] [00] [28] [F0] [14] [00] [AA] [BB] [CC] [DD]
        // [28] [F0] = CRC
        // [14] [00] = PC
        // [AA] [BB] [CC] [DD] = EPC

        // Bank 2
        // response: [00] [18] [28] [00] [00] [10] [00] [00] [E2] [00] [34] [12] [01] [6E] [FE] [00] [03] [7D] [9A] [A3] [28] [05] [01] [69] [10] [05] [5F] [F B] [FF] [FF] [DC] [00]
        // [E2] = CIsID
        // [00] [34] [12] = Vendor ID = 003, Model ID == 412
        // [01] [6E] [FE] [00] [03] [7D] [9A] [A3] [28] [05] [01] [69] [10] [05] [5F] [FB] [FF] [FF] [DC] [00] = Unique ID (TID)

        // Bank 3
        // response: [00] [40] [28] [00] [00] [10] [00] [00] [41] [43] [42] [44] [45] [46] [00] [00] [00] [00] [00] [00] ...
        // User data

        let mut data = [0u8; 11];

        // Insert timeout
        data[0..2].copy_from_slice(&time_out.to_be_bytes());

        // A previous version of this library did not include these 3 bytes. It works
        // fine with the M6E, but not the M7E. After reverse engineering the protocol
        // from the Mercury API (TMR_SR_cmdGEN2ReadTagData() in serial_reader_l3.c),
        // it was found that these 3 bytes are required. Not really sure what they do,
        // but it seems to work!
        data[2] = 0x10; // Option byte
        data[3] = 0x00; // Metadata MSB
        data[4] = 0x00; // Metadata LSB

        data[5] = bank; //Bank

        // Splice address into array
        data[6..10].copy_from_slice(&address.to_be_bytes());

        // The last byte is the number of 16-bit words to read. If it's set to zero,
        // then it will read the entire bank. We could set this to dataLengthRead / 2,
        // but it's easier to just set it to zero and truncate the response later.
        // That also helps if dataLengthRead differs from the actual the bank size,
        // which can cause the read to fail entirely.
        data[10] = 0x00;

        self.send_message(
            TMR_SR_OPCODE_READ_TAG_DATA,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );

        if self.response_error == Some(AllGood) {
            let status = u16::from_be_bytes([self.msg[3], self.msg[4]]);

            if status == 0x0000 {
                let response_length = self.msg[1] - 3;

                // Stop from reading more data than we have
                let data_length_read = min(usize::from(response_length), data_read.len());

                // There is a case here where responseLegnth is more than dataLengthRead, in which case we ignore (don't load) the additional bytes
                // Load limited response data into caller's array
                for x in 0..data_length_read {
                    // Data starts at byte 8 (header (1), size (1), opcode (1), status (2), option (1), metadata (2))
                    data_read[x] = self.msg[x + 8];
                }

                return Ok(data_length_read);
            }
        }

        *data_length_read = 0;

        Err(ResponseError::ResponseFail)
    }

    pub fn kill_tag(&mut self, password: &[u8], time_out: u16) -> ResponseError {
        let [msb_timeout, lsb_timeout] = time_out.to_be_bytes();

        let mut data = [0u8; 4 + password.len()];

        // Timeout
        data[0] = msb_timeout;
        data[1] = lsb_timeout;

        // Option initialize
        data[2] = 0x00;

        // Password
        data[3..3 + password.len()].copy_from_slice(password);

        // RFU
        data[3 + password.len()] = 0x00;

        self.send_message(
            TMR_SR_OPCODE_KILL_TAG,
            &data,
            u8::try_from(data.len()).unwrap(),
            time_out,
            true,
        );

        if self.response_error == Some(ResponseError::AllGood) {
            let status = u16::from_be_bytes([self.msg[3], self.msg[4]]);

            if status == 0x0000 {
                return ResponseError::ResponseSuccess;
            }
        }

        ResponseError::ResponseFail
    }

    pub fn check(&mut self) -> bool {
        let mut buf = [0u8; 1];

        while self.uart.read_ready() {
            self.uart.read(&mut buf).unwrap();

            if self.msg_head == 0 && buf[0] != 0xFF {
                // Ignore bytes because we need a start byte
            } else {
                self.msg[self.msg_head] = buf[0];
                self.msg_head += 1;

                self.msg_head %= MAX_MSG_SIZE;

                if self.msg_head > 0 && self.msg_head == (usize::from(self.msg[1]) + 7) {
                    // We have a complete sentence

                    // Erase rest of array
                    for x in self.msg_head..MAX_MSG_SIZE {
                        self.msg[x] = 0;
                    }

                    // Reset
                    self.msg_head = 0;

                    // Used for debugging: Does the user want us to print the command to serial port?
                    if self.print_debug
                        && let Some(debug_uart) = self.debug_uart.as_mut()
                    {
                        debug_uart.write(b"response: ");
                        self.print_message_array();
                    }
                    return true;
                }
            }
        }

        false
    }

    fn get_tag_epc_bytes(&self) -> u8 {
        let mut epcs_bits = 0;

        let tag_data_bytes = usize::from(self.get_tag_data_bytes());

        for x in 0..2usize {
            epcs_bits |= u16::from(self.msg[27 + tag_data_bytes + x]) << (8 * (1 - x));
        }

        let epc_bytes = epcs_bits / 8;
        // Ignore first and last two bytes
        u8::try_from(epc_bytes - 4).unwrap()
    }

    fn get_tag_data_bytes(&self) -> u8 {
        let tag_data_length = (u16::from(self.msg[24]) << 8) | u16::from(self.msg[25]);

        // If our data reading bits are messed up, we want to enter unrecoverable
        u8::try_from((tag_data_length + 7) / 8).unwrap()
    }

    pub fn get_tag_time_stamp(&self) -> u16 {
        let mut time_stamp = 0;

        for x in 0..4usize {
            time_stamp |= u32::from(self.msg[17 + x]) << (8 * (3 - x));
        }
        // If our data reading bits are messed up, we want to enter unrecoverable
        u16::try_from(time_stamp).unwrap()
    }

    pub fn get_tag_freq(&self) -> u32 {
        let mut freq = 0;

        for x in 0..3usize {
            freq |= u32::from(self.msg[x + 14]) << (8 * (2 - x));
        }

        freq
    }

    pub fn get_tag_rssi(&self) -> i8 {
        i8::from_ne_bytes([self.msg[12]])
    }

    pub fn parse_response(&mut self) -> ResponseError {
        // Taken from original source:

        //  See http://www.thingmagic.com/images/Downloads/Docs/AutoConfigTool_1.2-UserGuide_v02RevA.pdf
        //  for a breakdown of the response packet

        //  Example response:
        //  FF  28  22  00  00  10  00  1B  01  FF  01  01  C4  11  0E  16
        //  40  00  00  01  27  00  00  05  00  00  0F  00  80  30  00  00
        //  00  00  00  00  00  00  00  00  00  15  45  E9  4A  56  1D
        //  [0] FF = Header
        //  [1] 28 = Message length
        //  [2] 22 = OpCode
        //  [3, 4] 00 00 = Status
        //  [5 to 11] 10 00 1B 01 FF 01 01 = RFU 7 bytes
        //  [12] C4 = RSSI
        //  [13] 11 = Antenna ID (4MSB = TX, 4LSB = RX)
        //  [14, 15, 16] 0E 16 40 = Frequency in kHz
        //  [17, 18, 19, 20] 00 00 01 27 = Timestamp in ms since last keep alive msg
        //  [21, 22] 00 00 = phase of signal tag was read at (0 to 180)
        //  [23] 05 = Protocol ID
        //  [24, 25] 00 00 = Number of bits of embedded tag data [M bytes]
        //  [26 to M] (none) = Any embedded data
        //  [26 + M] 0F = RFU reserved future use
        //  [27, 28 + M] 00 80 = EPC Length [N bytes]  (bits in EPC including PC and CRC bits). 128 bits = 16 bytes
        //  [29, 30 + M] 30 00 = Tag EPC Protocol Control (PC) bits
        //  [31 to 42 + M + N] 00 00 00 00 00 00 00 00 00 00 15 45 = EPC ID
        //  [43, 44 + M + N] 45 E9 = EPC CRC
        //  [45, 46 + M + N] 56 1D = Message CRC

        let message_length = usize::from(self.msg[1] + 7);
        let opcode = self.msg[2];

        let message_crc = calculate_crc(&self.msg[1..], message_length - 3); //Ignore header (start spot 1), remove 3 bytes (header + 2 CRC)
        let [msb_crc, lsb_crc] = message_crc.to_be_bytes();
        if self.msg[message_length - 2] != msb_crc || self.msg[message_length - 1] != lsb_crc {
            return ResponseError::ErrorCorruptResponse;
        }

        if opcode == TMR_SR_OPCODE_READ_TAG_ID_MULTIPLE {
            // opcode 0x22

            //Based on the record length identify if this is a tag record, a temperature sensor record, or a keep-alive?
            if self.msg[1] == 0x00 {
                let mut status_message = 0;
                for x in 0..2usize {
                    status_message |= self.msg[x + 3] << (8 * (1 - x));
                }

                match status_message {
                    0x0400 => ResponseError::ResponseIsKeepAlive,
                    0x0504 => ResponseError::ResponseIsTempThrottle,
                    0x0505 => ResponseError::ResponseIsHighReturnLoss,
                    _ => ResponseError::ResponseIsUnknown,
                }
            } else if self.msg[1] == 0x08 {
                ResponseError::ResponseIsUnknown
            } else if self.msg[1] == 0x0a {
                ResponseError::ResponseIsTemperature
            } else {
                //This is a full tag response
                //User can now pull out RSSI, frequency of tag, timestamp, EPC, Protocol control bits, EPC CRC, CRC
                ResponseError::ResposneIsTagFound
            }
        } else {
            if self.print_debug
                && let Some(debug_uart) = self.debug_uart.as_mut()
            {
                debug_uart.write(b"Unknown opcode in response 0x");
                write_hex_byte(debug_uart, opcode);
            }
            ResponseError::ErrorUnknownOpcode
        }
    }

    fn send_message(
        &mut self,
        opcode: u8,
        data: &[u8],
        size: u8,
        time_out: u16,
        wait_for_response: bool,
    ) {
        self.msg[1] = size;
        self.msg[2] = opcode;

        for x in 0..size as usize {
            if let Some(data) = data.get(x) {
                self.msg[x + 3] = *data;
            }
        }

        self.send_command(time_out, wait_for_response);
    }

    fn send_command(&mut self, time_out: u16, wait_for_response: bool) {
        // Universal header
        self.msg[0] = 0xFF;
        let mut message_length = self.msg[1] as usize;
        // Used to check if response from module has the same opcode
        let opcode = self.msg[2];
        // Calculate CRC starting from index 1, and add 2 for LEN and OPCODE bytes.
        let mut crc = calculate_crc(&self.msg[1..], message_length + 2);

        let [msb_crc, lsb_crc] = crc.to_be_bytes();

        self.msg[message_length + 3] = msb_crc;
        self.msg[message_length + 4] = lsb_crc;

        if self.print_debug
            && let Some(debug_uart) = self.debug_uart.as_mut()
        {
            debug_uart.write(b"send_command: ");
            self.print_message_array();
        }

        let mut buf = [0u8; 1];

        while self.uart.read_ready() {
            let _ = self.uart.read(&mut buf);
        }

        for x in 0..message_length + 5 {
            self.uart.write(&[self.msg[x]]);
        }

        if !wait_for_response {
            self.uart.flush();
            return;
        }

        let start_time = Instant::now();

        while !self.uart.read_ready() {
            if start_time.elapsed() > Duration::from_millis(u64::from(time_out)) {
                if self.print_debug
                    && let Some(debug_uart) = self.debug_uart.as_mut()
                {
                    debug_uart.write(b"Time out 1: No response from module\r\n");
                }
                self.response_error = Some(ResponseError::ErrorCommandResponseTimeout);
                return;
            }
            todo!("1ms sleep");
        }

        message_length = MAX_MSG_SIZE - 1;
        let mut spot = 0;
        let mut buf = [0u8; 1];

        while spot < message_length {
            if start_time.elapsed() > Duration::from_millis(u64::from(time_out)) {
                if self.print_debug
                    && let Some(debug_uart) = self.debug_uart.as_mut()
                {
                    debug_uart.write(b"Time out 2: No response from module\r\n");
                }
                self.response_error = Some(ResponseError::ErrorCommandResponseTimeout);
                return;
            }

            if self.uart.read_ready() {
                self.uart.read(&mut buf).unwrap();
                self.msg[spot] = buf[0];
                if spot == 1 {
                    message_length = usize::from(self.msg[1]) + 7;
                }

                spot += 1;
                spot %= MAX_MSG_SIZE;
            }
        }
        if self.print_debug
            && let Some(debug_uart) = self.debug_uart.as_mut()
        {
            debug_uart.write(b"response: ");
            self.print_message_array();
        }

        crc = calculate_crc(&self.msg[1..], message_length - 3);
        let [msb_crc, lsb_crc] = crc.to_be_bytes();
        if self.msg[message_length - 2] != msb_crc || self.msg[message_length] != lsb_crc {
            self.response_error = Some(ResponseError::ErrorCorruptResponse);
            if self.print_debug
                && let Some(debug_uart) = self.debug_uart.as_mut()
            {
                debug_uart.write(b"Corrupt response");
            }
            return;
        }

        if self.msg[2] != opcode {
            self.response_error = Some(ResponseError::ErrorWrongOpcodeResponse);
            if self.print_debug
                && let Some(debug_uart) = self.debug_uart.as_mut()
            {
                debug_uart.write(b"Wrong opcode response");
            }
            return;
        }

        self.response_error = Some(ResponseError::AllGood);
    }

    fn print_message_array(&mut self) {
        if self.print_debug
            && let Some(debug_uart) = self.debug_uart.as_mut()
        {
            let amount_to_print = (usize::from(self.msg[1]) + 5).min(MAX_MSG_SIZE);

            for x in 0..amount_to_print {
                debug_uart.write(b" [");

                let byte = self.msg[x];

                if byte < 0x10 {
                    debug_uart.write(b"0");
                }

                write_hex_byte(debug_uart, byte);
                debug_uart.write(b"]");
            }

            debug_uart.write(b"\r\n");
        }
    }
}

fn calculate_crc(buf: &[u8], len: usize) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for i in 0..len {
        crc = ((crc << 4) | (u16::from((*buf)[i]) >> 4)) ^ CRC_TABLE[usize::from(crc >> 12)];

        crc = ((crc << 4) | (u16::from((*buf)[i]) & 0x0F)) ^ CRC_TABLE[usize::from(crc >> 12)];
    }

    crc
}

fn write_hex_byte(uart: &mut Uart<'static, esp_hal::Blocking>, byte: u8) {
    let hex = b"0123456789ABCDEF";
    let chars = [hex[usize::from(byte >> 4)], hex[usize::from(byte & 0x0F)]];

    let _ = uart.write(&chars);
}
