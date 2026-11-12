use core::{cell::RefCell, sync::atomic::{AtomicU16, AtomicU32, Ordering}};

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex};
use embedded_sdmmc::{TimeSource, Timestamp};

use crate::{rfid_task::EPC, time_source::DeviceTimeSource};

// Store atomically for safety between async tasks
static BATS_READ: AtomicU32 = AtomicU32::new(0);

pub fn increment_bats() {
    // We don't care about what the value was
    let _ = BATS_READ.fetch_add(1, Ordering::SeqCst);
}

pub fn get_current_bat_amount() -> u32 {
    BATS_READ.load(Ordering::SeqCst)
}

static LAST_RFID_READ: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Option<EPC>> = embassy_sync::mutex::Mutex::new(Option::None);

pub async fn update_most_recent_epc(epc: &EPC) {
    *LAST_RFID_READ.lock().await = Some(epc.clone());
}

pub async fn get_most_recent_epc() -> Option<EPC> {
    LAST_RFID_READ.lock().await.clone()
}

static POWER_LEVEL: AtomicU16 = AtomicU16::new(0);

pub fn set_power_level(power_level: u16) {
    POWER_LEVEL.store(power_level, Ordering::SeqCst);
}

pub fn get_current_power_level() -> u16 {
    POWER_LEVEL.load(Ordering::SeqCst)
}

// Stores temperature in fahrenheit * 10
static RFID_MODULE_TEMPERATURE: AtomicU16 = AtomicU16::new(0);

// Might eventually change input. Not sure how to get temp still
pub fn set_rfid_module_temperature(temp_f_times_10: u16) {
    RFID_MODULE_TEMPERATURE.store(temp_f_times_10, Ordering::SeqCst);
}

pub fn get_current_rfid_module_temperature() -> f32 {
    let temp_f_times_10 = RFID_MODULE_TEMPERATURE.load(Ordering::SeqCst);
    (temp_f_times_10 as f32) / 10.0f32
}

static DEVICE_TIME_SOURCE: embassy_sync::blocking_mutex::Mutex<
    CriticalSectionRawMutex,
    RefCell<DeviceTimeSource>,
> = embassy_sync::blocking_mutex::Mutex::new(
    RefCell::new(DeviceTimeSource::new())
);

pub struct SharedDeviceTimeSource;

impl TimeSource for SharedDeviceTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        DEVICE_TIME_SOURCE.lock(|source| {
            source.borrow().get_timestamp()
        })
    }
}

pub fn update_device_time(timestamp: Timestamp) {
    DEVICE_TIME_SOURCE.lock(|source| {
        source.borrow_mut().set_time(timestamp);
    });
}

pub fn get_current_time() -> Timestamp {
    DEVICE_TIME_SOURCE.lock(|source| {
        source.borrow().get_timestamp()
    })
}
