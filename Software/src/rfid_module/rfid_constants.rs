/*

This code is based in part on the SparkFun Simultaneous RFID Tag Reader
library by Nathan Seidle / SparkFun Electronics.

Copyright (c) 2016 SparkFun Electronics

Licensed under the MIT License.

*/

pub(crate) const MAX_MSG_SIZE: usize = 255;

pub(crate) const TMR_SR_OPCODE_VERSION: u8 = 0x03;
pub(crate) const TMR_SR_OPCODE_SET_BAUD_RATE: u8 = 0x06;
pub(crate) const TMR_SR_OPCODE_READ_TAG_ID_SINGLE: u8 = 0x21;
pub(crate) const TMR_SR_OPCODE_READ_TAG_ID_MULTIPLE: u8 = 0x22;
pub(crate) const TMR_SR_OPCODE_WRITE_TAG_ID: u8 = 0x23;
pub(crate) const TMR_SR_OPCODE_WRITE_TAG_DATA: u8 = 0x24;
pub(crate) const TMR_SR_OPCODE_KILL_TAG: u8 = 0x26;
pub(crate) const TMR_SR_OPCODE_READ_TAG_DATA: u8 = 0x28;
pub(crate) const TMR_SR_OPCODE_CLEAR_TAG_ID_BUFFER: u8 = 0x2A;
pub(crate) const TMR_SR_OPCODE_MULTI_PROTOCOL_TAG_OP: u8 = 0x2F;
pub(crate) const TMR_SR_OPCODE_GET_READ_TX_POWER: u8 = 0x62;
pub(crate) const TMR_SR_OPCODE_GET_WRITE_TX_POWER: u8 = 0x64;
pub(crate) const TMR_SR_OPCODE_GET_USER_GPIO_INPUTS: u8 = 0x66;
pub(crate) const TMR_SR_OPCODE_GET_POWER_MODE: u8 = 0x68;
pub(crate) const TMR_SR_OPCODE_GET_READER_OPTIONAL_PARAMS: u8 = 0x6A;
pub(crate) const TMR_SR_OPCODE_GET_PROTOCOL_PARAM: u8 = 0x6B;
pub(crate) const TMR_SR_OPCODE_SET_ANTENNA_PORT: u8 = 0x91;
pub(crate) const TMR_SR_OPCODE_SET_TAG_PROTOCOL: u8 = 0x93;
pub(crate) const TMR_SR_OPCODE_SET_READ_TX_POWER: u8 = 0x92;
pub(crate) const TMR_SR_OPCODE_SET_WRITE_TX_POWER: u8 = 0x94;
pub(crate) const TMR_SR_OPCODE_SET_USER_GPIO_OUTPUTS: u8 = 0x96;
pub(crate) const TMR_SR_OPCODE_SET_REGION: u8 = 0x97;
pub(crate) const TMR_SR_OPCODE_SET_READER_OPTIONAL_PARAMS: u8 = 0x9A;
pub(crate) const TMR_SR_OPCODE_SET_PROTOCOL_PARAM: u8 = 0x9B;
pub(crate) const COMMAND_TIME_OUT: u8 = 2000; // Number of ms before stop waiting for response from module

// Define all the ways functions can return
pub(crate) enum ReturnType {
    AllGood,
    ErrorCommandResponseTimeout,
    ErrorCorruptResponse,
    ErrorWrongOpcodeResponse,
    ErrorUnknownOpcode,
    ResponseIsTemperature,
    ResponseIsKeepAlive,
    ResponseIsTempThrottle,
    ResposneIsTagFound,
    ResponseIsNotAGFound,
    ResponseIsUnknown,
    ResponseSuccess,
    ResponseFail,
    ResponseIsHighReturnLoss,
}

// Define the allowed regions - these set the internal freq of the module
// pub(crate) enum AllowedRegions {
//     RegionNorthAmerica,
//     RegionIndia,
//     RegionJapan,
//     RegionChina,
//     RegionEurope,
//     RegionKorea,
//     RegionAustralia,
//     RegionNewZealand,
//     RegionNorthAmerica2,
//     RegionNorthAmerica3,
//     RegionOpen,
// }
pub(crate) const REGION_NORTHAMERICA: u8 = 0x01;
pub(crate) const REGION_INDIA: u8 = 0x04;
pub(crate) const REGION_JAPAN: u8 = 0x05;
pub(crate) const REGION_CHINA: u8 = 0x06;
pub(crate) const REGION_EUROPE: u8 = 0x08;
pub(crate) const REGION_KOREA: u8 = 0x09;
pub(crate) const REGION_AUSTRALIA: u8 = 0x0B;
pub(crate) const REGION_NEWZEALAND: u8 = 0x0C;
pub(crate) const REGION_NORTHAMERICA2: u8 = 0x0D;
pub(crate) const REGION_NORTHAMERICA3: u8 = 0x0E;
pub(crate) const REGION_OPEN: u8 = 0xFF;

enum PinMode {
    INPUT,
    OUPUT,
}