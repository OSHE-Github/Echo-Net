use embedded_sdmmc::Timestamp;
use heapless::{String, format};

use crate::{
    cross_task_data::{
        get_current_bat_amount, get_current_power_level, get_current_time, get_most_recent_epc, set_power_level, update_device_time,
    }, sd_card_task::format_epc, time_source::DeviceTimeSource,
};

struct MainMenu {
    // Bat amount read this session
    // Last RFID read
    // Power Level
}

impl MainMenu {
    pub fn new() -> Self {
        Self {}
    }
}

struct PowerMenu {
    // "Power Level"
    // changing power level
    power_level_digit: usize,
    power_digits: [u8; 4],
}

impl PowerMenu {
    pub fn new(mut power_level: u16) -> Self {
        let mut power_digits = [0u8; 4];
        for i in (0..4).rev() {
            power_digits[i] = (power_level % 10) as u8;
            power_level /= 10;
        }
        Self {
            power_level_digit: 0,
            power_digits,
        }
    }

    pub fn increase_power_on_digit(&mut self) {
        self.power_digits[self.power_level_digit] += 1;
    }

    pub fn increase_digit(&mut self) {
        self.power_level_digit += 1;
        self.power_level_digit %= 4;
    }

    pub fn get_power_level(&self) -> u16 {
        let mut power_level = 0u16;
        for digit in self.power_digits {
            power_level = power_level * 10 + digit as u16;
        }

        power_level
    }
}

struct ReadTagMenu {
    // "RFID Tags"
    // Scrolling tags
    tag_index: usize,
}

impl ReadTagMenu {
    pub fn new() -> Self {
        Self { tag_index: 0 }
    }

    pub fn increase_index(&mut self) {
        self.tag_index += 1;
    }

    pub fn decrease_index(&mut self) {
        self.tag_index -= 1;
    }

    pub fn get_index(&self) -> usize {
        self.tag_index
    }
}

struct TempMenu {
    // "Temperature"
    // temperature
}

struct TimeMenu {
    // Currently selected digit (0-7)
    time_index: usize,

    // YY MM DD HH MM
    time_digits: [u8; 10],
}

impl TimeMenu {
    pub fn new() -> Self {
        Self {
            time_index: 0,
            time_digits: [0; 10],
        }
    }

    pub fn increase_digit(&mut self) {
        self.time_index = (self.time_index + 1) % 10;
    }

    pub fn increase_time_on_digit(&mut self) {
        let max_values = [1, 9, 9, 9, 9, 9, 2, 3, 5, 9];

        self.time_digits[self.time_index] =
            (self.time_digits[self.time_index] + 1) % (max_values[self.time_index] + 1);

        self.update_timestamp();
    }

    pub fn get_time(&self) -> String<16> {
        let time = format!(16;
            "{:02}/{:02}/{:02},{:02}:{:02}",
            self.time_digits[0] * 10 + self.time_digits[1],
            self.time_digits[2] * 10 + self.time_digits[3],
            self.time_digits[4] * 10 + self.time_digits[5],
            self.time_digits[6] * 10 + self.time_digits[7],
            self.time_digits[8] * 10 + self.time_digits[9],
        )
        .unwrap();

        time
    }

    pub fn update_timestamp(&self) {
        let month = self.time_digits[0] * 10 + self.time_digits[1];
        let day = self.time_digits[2] * 10 + self.time_digits[3];
        let year = self.time_digits[4] * 10 + self.time_digits[5];
        let hour = self.time_digits[6] * 10 + self.time_digits[7];
        let minute = self.time_digits[8] * 10 + self.time_digits[9];

        update_device_time(Timestamp {
            year_since_1970: year.saturating_add(30),
            zero_indexed_month: month.saturating_sub(1),
            zero_indexed_day: day.saturating_sub(1),
            hours: hour,
            minutes: minute,
            seconds: 0,
        });
    }
}

pub struct UIMenu {
    active_menu_state: ActiveMenuState,

    // We only have state data on some states
    power_menu: PowerMenu,
    tag_menu: ReadTagMenu,
    time_menu: TimeMenu,
}

enum ActiveMenuState {
    Main,
    Power,
    ReadTags,
    Temp,
    Time,
}

impl UIMenu {
    pub fn new() -> Self {
        Self {
            // Start on the main screen
            active_menu_state: ActiveMenuState::Main,
            // State data
            power_menu: PowerMenu::new(get_current_power_level()),
            tag_menu: ReadTagMenu::new(),
            time_menu: TimeMenu::new(),
        }
    }

    // Move to the next menu mode
    pub fn next(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::Main => self.active_menu_state = ActiveMenuState::Power,
            ActiveMenuState::Power => self.active_menu_state = ActiveMenuState::ReadTags,
            ActiveMenuState::ReadTags => self.active_menu_state = ActiveMenuState::Temp,
            ActiveMenuState::Temp => self.active_menu_state = ActiveMenuState::Time,
            ActiveMenuState::Time => self.active_menu_state = ActiveMenuState::Main,
        }
    }

    pub async fn get_display(&self) -> [Option<String<16>>; 3] {
        match self.active_menu_state {
            ActiveMenuState::Main => self.get_main_menu_text().await,
            ActiveMenuState::Power => self.get_power_menu_text(),
            ActiveMenuState::ReadTags => self.get_tag_menu_text(),
            ActiveMenuState::Temp => self.get_temp_menu_text(),
            ActiveMenuState::Time => self.get_time_menu_text().await,
        }
    }

    pub fn press_trigger_button(&mut self) {
        // Currently doesn't do anything
    }

    pub fn press_menu_button(&mut self) {
        self.next();
    }

    pub fn press_plus_button(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::Power => {
                self.power_menu.increase_power_on_digit();
            }
            ActiveMenuState::ReadTags => {
                self.tag_menu.increase_index();
            }
            ActiveMenuState::Time => {
                self.time_menu.increase_time_on_digit();
            }
            _ => {}
        }
        self.set_power_level_ui();
    }

    pub fn press_lock_button(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::Power => {
                self.power_menu.increase_digit();
            }
            ActiveMenuState::ReadTags => {
                self.tag_menu.decrease_index();
            }
            ActiveMenuState::Time => {
                self.time_menu.increase_digit();
            }
            _ => {}
        }
    }

    pub async fn get_main_menu_text(&self) -> [Option<String<16>>; 3] {
        let string_1 = format!(16; "{} bats read", get_current_bat_amount()).unwrap();
        let epc = get_most_recent_epc().await;
        let string_2 = match epc {
            Some(epc) => format_epc(&epc),
            None => format!(16; "no recent tags").unwrap(),
        };
        let string_3 = format!(16; "power: {}", get_current_power_level()).unwrap();

        [Some(string_1), Some(string_2), Some(string_3)]
    }

    pub fn get_power_menu_text(&self) -> [Option<String<16>>; 3] {
        let header_string = format!(16; "Power Level").unwrap();
        let power_level_string = format!(16; "{:04}", get_current_power_level()).unwrap();
        let cursor_string = match self.power_menu.power_level_digit {
            0 => format!(16; "^   "),
            1 => format!(16; " ^  "),
            2 => format!(16; "  ^ "),
            _ => format!(16; "   ^"),
        }.unwrap();

        [Some(header_string), Some(power_level_string), Some(cursor_string)]
    }

    pub fn set_power_level_ui(&self) {
        set_power_level(self.power_menu.get_power_level());
    }

    pub fn get_tag_menu_text(&self) -> [Option<String<16>>; 3] {
        let header = format!(16; "RFID Tags").unwrap();

        // We need a scrolling list of RFID tags
        [Some(header), None, None]
    }

    pub fn get_temp_menu_text(&self) -> [Option<String<16>>; 3] {
        let header = format!(16; "Temperature").unwrap();
        let temperature = format!(16; "0.0f").unwrap();

        [Some(header), Some(temperature), None]
    }

    async fn get_time_menu_text(&self) -> [Option<String<16>>; 3] {
        let header = format!(16; "Time").unwrap();
        let time = get_current_time().clone();
        let time_since_2000 = time.year_since_1970.saturating_sub(30);
        // let time_string = format!(16; "{:02}/{:02}", time.zero_indexed_month+1, time_since_2000).unwrap();
        let time_string = format!(16; "{:02}/{:02}/{:02},{:02}:{:02}", 
            time.zero_indexed_month.saturating_add(1), time.zero_indexed_day.saturating_add(1), time_since_2000, time.hours, time.minutes).unwrap_or(format!(16; " a ").unwrap());

        let cursor_string = match self.time_menu.time_index {
            0 => format!(16; "^             "),
            1 => format!(16; " ^            "),
            2 => format!(16; "   ^          "),
            3 => format!(16; "    ^         "),
            4 => format!(16; "      ^       "),
            5 => format!(16; "       ^      "),
            6 => format!(16; "         ^    "),
            7 => format!(16; "          ^   "),
            8 => format!(16; "            ^ "),
            _ => format!(16; "             ^"),
        }.unwrap();

        [Some(header), Some(time_string), Some(cursor_string)]
    }
}
