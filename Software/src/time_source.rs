// Dummy time source for SD card manager. Doesn't actually give accurate data.

use embedded_sdmmc::{TimeSource, Timestamp};

pub struct DummyTimeSource;

impl TimeSource for DummyTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 56,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}