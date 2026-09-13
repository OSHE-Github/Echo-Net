/*

This code is based in part on the SparkFun Simultaneous RFID Tag Reader
library by Nathan Seidle / SparkFun Electronics.

Copyright (c) 2016 SparkFun Electronics

Licensed under the MIT License.

*/

pub(crate) const MAX_MSG_SIZE: usize = 255;

// Some of these opcodes are never used, but I'd like to wait to delete them until we have the rfid module working, just in case.

pub(crate) const TMR_SR_OPCODE_VERSION: u8 = 0x03;
pub(crate) const TMR_SR_OPCODE_SET_BAUD_RATE: u8 = 0x06;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_READ_TAG_ID_SINGLE: u8 = 0x21;
pub(crate) const TMR_SR_OPCODE_READ_TAG_ID_MULTIPLE: u8 = 0x22;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_WRITE_TAG_ID: u8 = 0x23;
pub(crate) const TMR_SR_OPCODE_WRITE_TAG_DATA: u8 = 0x24;
pub(crate) const TMR_SR_OPCODE_KILL_TAG: u8 = 0x26;
pub(crate) const TMR_SR_OPCODE_READ_TAG_DATA: u8 = 0x28;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_CLEAR_TAG_ID_BUFFER: u8 = 0x2A;
pub(crate) const TMR_SR_OPCODE_MULTI_PROTOCOL_TAG_OP: u8 = 0x2F;
pub(crate) const TMR_SR_OPCODE_GET_READ_TX_POWER: u8 = 0x62;
pub(crate) const TMR_SR_OPCODE_GET_WRITE_TX_POWER: u8 = 0x64;
pub(crate) const TMR_SR_OPCODE_GET_USER_GPIO_INPUTS: u8 = 0x66;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_GET_POWER_MODE: u8 = 0x68;
pub(crate) const TMR_SR_OPCODE_GET_READER_OPTIONAL_PARAMS: u8 = 0x6A;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_GET_PROTOCOL_PARAM: u8 = 0x6B;
pub(crate) const TMR_SR_OPCODE_SET_ANTENNA_PORT: u8 = 0x91;
pub(crate) const TMR_SR_OPCODE_SET_TAG_PROTOCOL: u8 = 0x93;
pub(crate) const TMR_SR_OPCODE_SET_READ_TX_POWER: u8 = 0x92;
pub(crate) const TMR_SR_OPCODE_SET_WRITE_TX_POWER: u8 = 0x94;
pub(crate) const TMR_SR_OPCODE_SET_USER_GPIO_OUTPUTS: u8 = 0x96;
pub(crate) const TMR_SR_OPCODE_SET_REGION: u8 = 0x97;
pub(crate) const TMR_SR_OPCODE_SET_READER_OPTIONAL_PARAMS: u8 = 0x9A;
#[allow(unused)]
pub(crate) const TMR_SR_OPCODE_SET_PROTOCOL_PARAM: u8 = 0x9B;
pub(crate) const COMMAND_TIME_OUT: u16 = 2000; // Number of ms before stop waiting for response from module

pub const DEFAULT_BAUD_RATE: u32 = 115200;

// Define all the ways functions can return
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseError {
    AllGood,
    ErrorCommandResponseTimeout,
    ErrorCorruptResponse,
    ErrorWrongOpcodeResponse,
    ErrorUnknownOpcode,
    ErrorBufferLength,
    ResponseIsTemperature,
    ResponseIsKeepAlive,
    ResponseIsTempThrottle,
    ResposneIsTagFound,
    ResponseIsNotAGFound,
    ResponseIsUnknown,
    ResponseFail,
    ResponseIsHighReturnLoss,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Region {
    NorthAmerica,
    India,
    Japan,
    China,
    Europe,
    Korea,
    Australia,
    NewZealand,
    NorthAmerica2,
    NorthAmerica3,
    Open,
}

impl From<Region> for u8 {
    fn from(region: Region) -> Self {
        match region {
            Region::NorthAmerica => 0x01,
            Region::India => 0x04,
            Region::Japan => 0x05,
            Region::China => 0x06,
            Region::Europe => 0x08,
            Region::Korea => 0x09,
            Region::Australia => 0x0B,
            Region::NewZealand => 0x0C,
            Region::NorthAmerica2 => 0x0D,
            Region::NorthAmerica3 => 0x0E,
            Region::Open => 0xFF,
        }
    }
}

pub(crate) const CRC_TABLE: [u16; 16] = [
    0x0000, 0x1021, 0x2042, 0x3063, 0x4084, 0x50a5, 0x60c6, 0x70e7, 0x8108, 0x9129, 0xa14a, 0xb16b,
    0xc18c, 0xd1ad, 0xe1ce, 0xf1ef,
];

// Could be made bigger, not really sure if we need to though
pub(crate) const MAX_DATA_TO_RECORD: usize = 2056usize;
pub(crate) const MAX_PASSWORD_SIZE: usize = 256usize;

pub enum RFIDPinMode {
    INPUT,
    OUPUT,
}

pub enum RFIDPinState {
    ON,
    OFF,
}

pub(crate) fn get_pin_state(pin: u8) -> Option<RFIDPinState> {
    match pin {
        0 => Some(RFIDPinState::OFF),
        1 => Some(RFIDPinState::ON),
        // The pin is not a valid state
        _ => None,
    }
}

impl Into<u8> for RFIDPinMode {
    fn into(self) -> u8 {
        match self {
            Self::INPUT => 0u8,
            Self::OUPUT => 1u8,
        }
    }
}
