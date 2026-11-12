// Dummy time source for SD card manager. Doesn't actually give accurate data.

use embassy_time::Instant;
use embedded_sdmmc::{TimeSource, Timestamp};

pub struct DeviceTimeSource {
    timestamp: Timestamp,
    time_created: Option<Instant>,
}

impl DeviceTimeSource {
    pub const fn new() -> Self {
        // Default as 2026 Jan 1st 11:11am
        Self {
            timestamp: Timestamp {
                year_since_1970: 56,
                zero_indexed_month: 0,
                zero_indexed_day: 0,
                hours: 11,
                minutes: 11,
                seconds: 0
            },
            time_created: None,
        }
    }

    pub fn set_time(&mut self, new_timestamp: Timestamp) {
        self.timestamp = new_timestamp;
        self.time_created = Some(Instant::now());
    } 

    fn is_leap_year(&self, year_since_1970: u8) -> bool {
        let year = 1970u16 + year_since_1970 as u16;

        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }

    fn days_in_month(&self, month: u8, year_since_1970: u8) -> u8 {
        match month {
            0 | 2 | 4 | 6 | 7 | 9 | 11 => 31,
            3 | 5 | 8 | 10 => 30,
            1 => {
                if self.is_leap_year(year_since_1970) {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    }

    fn add_seconds(&self, mut timestamp: Timestamp, elapsed: u64) -> Timestamp {
        let seconds = timestamp.seconds as u64 + elapsed;
        timestamp.seconds = (seconds % 60) as u8;

        let minutes = timestamp.minutes as u64 + seconds / 60;
        timestamp.minutes = (minutes % 60) as u8;

        let hours = timestamp.hours as u64 + minutes / 60;
        timestamp.hours = (hours % 24) as u8;

        let mut days = hours / 24;

        while days > 0 {
            let month_length = self.days_in_month(
                timestamp.zero_indexed_month,
                timestamp.year_since_1970,
            ) as u64;

            let remaining = month_length - timestamp.zero_indexed_day as u64 - 1;

            if days <= remaining {
                timestamp.zero_indexed_day += days as u8;
                break;
            }

            days -= remaining + 1;
            timestamp.zero_indexed_day = 0;

            if timestamp.zero_indexed_month == 11 {
                if timestamp.year_since_1970 == u8::MAX {
                    // Timestamp cannot represent years beyond 2225.
                    timestamp.zero_indexed_month = 11;
                    timestamp.zero_indexed_day = 30;
                    timestamp.hours = 23;
                    timestamp.minutes = 59;
                    timestamp.seconds = 59;
                    break;
                }

                timestamp.zero_indexed_month = 0;
                timestamp.year_since_1970 += 1;
            } else {
                timestamp.zero_indexed_month += 1;
            }
        }

        timestamp
    }
}

impl TimeSource for DeviceTimeSource {
    fn get_timestamp(&self) -> Timestamp {
        match self.time_created {
            Some(time_created) => {
                let elapsed = (Instant::now() - time_created).as_secs();
                self.add_seconds(self.timestamp, elapsed)
            }
            None => self.timestamp,
        }
    }
}