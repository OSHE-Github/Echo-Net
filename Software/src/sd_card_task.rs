use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Receiver};

use crate::rfid_task::RFIDData;

#[embassy_executor::task]
pub async fn rfid_task(rfid_data_reciever: Receiver<'static, CriticalSectionRawMutex, RFIDData, 8>) {

}