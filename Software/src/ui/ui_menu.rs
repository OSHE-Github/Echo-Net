use crate::{
    cross_task_data::{
        get_current_bat_amount, get_current_power_level, get_current_time,
        set_power_level, update_device_time,
    },
    ui::MENU_LINE_SIZE,
};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Sender},
};
use embedded_sdmmc::Timestamp;
use esp_println::println;
use heapless::{String, format};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum RestoreSessionOption {
    RestoreSession,
    NewSession,
}

struct RestoreSessionMenu {
    session_choice: RestoreSessionOption,
    epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
}

impl RestoreSessionMenu {
    pub fn new(epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>) -> Self {
        Self {
            session_choice: RestoreSessionOption::RestoreSession,
            epc_tag_request_sender,
        }
    }

    pub fn change_option(&mut self) {
        match self.session_choice {
            RestoreSessionOption::RestoreSession => self.session_choice = RestoreSessionOption::NewSession,
            RestoreSessionOption::NewSession => self.session_choice = RestoreSessionOption::RestoreSession,
        }
    }
}

struct PowerMenu {
    power_level_digit: usize,
    power_digits: [u8; 2],
}

impl PowerMenu {
    pub fn new(power_level: u16) -> Self {
        let mut new_power_level = power_level / 100;
        let mut power_digits = [0u8; 2];
        for i in (0..2).rev() {
            power_digits[i] = (new_power_level % 10) as u8;
            new_power_level /= 10;
        }
        Self {
            power_level_digit: 0,
            power_digits,
        }

    }

    pub fn increase_power_on_digit(&mut self) {
        let max_values = [2, 9];

        self.power_digits[self.power_level_digit] =
            (self.power_digits[self.power_level_digit] + 1) % (max_values[self.power_level_digit] + 1);
    }

    pub fn increase_digit(&mut self) {
        self.power_level_digit += 1;
        self.power_level_digit %= 2;
    }

    pub fn get_power_level(&self) -> u16 {
        let mut power_level = 0u16;
        for digit in self.power_digits {
            power_level = power_level * 10 + digit as u16;
        }

        power_level
    }
}

struct TimeMenu {
    // Currently selected field (0-4)
    time_index: usize,

    // MM/DD/YY,HH:MM
    time_values: [u8; 5],
}

impl TimeMenu {
    pub fn new() -> Self {
        Self {
            time_index: 0,
            time_values: [1, 1, 0, 0, 0],
        }
    }

    pub fn increase_digit(&mut self) {
        self.time_index = (self.time_index + 1) % 5;
    }

    pub fn increase_time_on_digit(&mut self) {
        let max_values = [12, 31, 99, 23, 59];
        let min_values = [1, 1, 0, 0, 0];

        let index = self.time_index;

        self.time_values[index] += 1;

        if self.time_values[index] > max_values[index] {
            self.time_values[index] = min_values[index];
        }

        self.update_timestamp();
    }

    pub fn update_timestamp(&self) {
        update_device_time(Timestamp {
            year_since_1970: self.time_values[2].saturating_add(30),
            zero_indexed_month: self.time_values[0].saturating_sub(1),
            zero_indexed_day: self.time_values[1].saturating_sub(1),
            hours: self.time_values[3],
            minutes: self.time_values[4],
            seconds: 0,
        });
    }
}

pub struct UIMenu {
    active_menu_state: ActiveMenuState,

    trigger_is_on: bool,

    // We only have state data on some states
    restore_session_menu: RestoreSessionMenu,
    power_menu: PowerMenu,
    time_menu: TimeMenu,
}

enum ActiveMenuState {
    // When the device is loading on
    Loading,
    // Ask user if they want to restore from last session
    RestoreLastSession,
    // Regular menus
    Main,
    Power,
    Time,
}

impl UIMenu {
    pub fn new(
        epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
    ) -> Self {
        Self {
            // Start on the main screen
            active_menu_state: ActiveMenuState::Loading,
            trigger_is_on: false,
            // State data
            restore_session_menu: RestoreSessionMenu::new(epc_tag_request_sender),
            power_menu: PowerMenu::new(get_current_power_level()),
            time_menu: TimeMenu::new(),
        }
    }

    pub fn load(&mut self) {
        self.active_menu_state = ActiveMenuState::RestoreLastSession;
    }

    // Move to the next menu mode
    pub async fn next(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::Loading => self.active_menu_state = ActiveMenuState::Loading,
            ActiveMenuState::RestoreLastSession => {
                if self.restore_session_menu.session_choice == RestoreSessionOption::RestoreSession {
                    println!("Restoring last session!");
                    self.restore_session_menu.epc_tag_request_sender.send(None).await;
                }
                self.active_menu_state = ActiveMenuState::Main
            },
            ActiveMenuState::Main => self.active_menu_state = ActiveMenuState::Power,
            ActiveMenuState::Power => self.active_menu_state = ActiveMenuState::Time,
            ActiveMenuState::Time => self.active_menu_state = ActiveMenuState::Main,
        }
    }

    pub async fn get_display(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        match self.active_menu_state {
            ActiveMenuState::Loading => self.get_loading_text(),
            ActiveMenuState::RestoreLastSession => self.get_restore_session_test(),
            ActiveMenuState::Main => self.get_main_menu_text().await,
            ActiveMenuState::Power => self.get_power_menu_text(),
            ActiveMenuState::Time => self.get_time_menu_text().await,
        }
    }

    pub fn press_trigger_button(&mut self) {
        self.trigger_is_on = !self.trigger_is_on;
    }

    pub async fn press_menu_button(&mut self) {
        self.next().await;
    }

    pub fn press_plus_button(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::RestoreLastSession => {
                self.restore_session_menu.change_option();
            }
            ActiveMenuState::Power => {
                self.power_menu.increase_power_on_digit();
            }
            ActiveMenuState::Time => {
                self.time_menu.increase_time_on_digit();
            }
            _ => {}
        }
        self.set_power_level_ui();
    }

    pub async fn press_lock_button(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::RestoreLastSession => {
                self.next().await;
            }
            ActiveMenuState::Power => {
                self.power_menu.increase_digit();
            }
            ActiveMenuState::Time => {
                self.time_menu.increase_digit();
            }
            _ => {}
        }
    }
    pub fn get_loading_text(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        let string_1 = format!(MENU_LINE_SIZE; "Starting up...").unwrap();
        [None, Some(string_1), None]
    }

    pub fn get_restore_session_test(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        let string_1 = format!(MENU_LINE_SIZE; "Restore session?").unwrap();
        let string_2 = 
        if self.restore_session_menu.session_choice == RestoreSessionOption::RestoreSession {
            format!(MENU_LINE_SIZE; "Yes <")
        } else {
            format!(MENU_LINE_SIZE; "Yes")
        }.unwrap();
        let string_3 = 
        if self.restore_session_menu.session_choice == RestoreSessionOption::NewSession {
            format!(MENU_LINE_SIZE; "New session <")
        } else {
            format!(MENU_LINE_SIZE; "New session")
        }.unwrap();
        [Some(string_1), Some(string_2), Some(string_3)]
    }

    pub async fn get_main_menu_text(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        let string_1 = format!(MENU_LINE_SIZE; "{} bats read", get_current_bat_amount()).unwrap();
        let power = get_current_power_level() / 100;
        // println!(" a: {}, b: {}", get_current_power_level(), power);
        let string_2 = format!(MENU_LINE_SIZE; "power: {}dB", power).unwrap();
        let string_3 = if self.trigger_is_on {
            Some(format!(MENU_LINE_SIZE; "reading...").unwrap())
        } else {
            None
        };

        [Some(string_1), Some(string_2), string_3]
    }

    pub fn get_power_menu_text(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        let header_string = format!(MENU_LINE_SIZE; "Power Level").unwrap();
        let power_level_string = format!(MENU_LINE_SIZE; "{:02}dB", get_current_power_level() / 100).unwrap();
        let cursor_string = match self.power_menu.power_level_digit {
            0 => format!(MENU_LINE_SIZE; "^   "),
            _ => format!(MENU_LINE_SIZE; " ^  "),
        }
        .unwrap();

        [
            Some(header_string),
            Some(power_level_string),
            Some(cursor_string),
        ]
    }

    pub fn set_power_level_ui(&self) {
        set_power_level(self.power_menu.get_power_level() * 100);
    }

    async fn get_time_menu_text(&self) -> [Option<String<MENU_LINE_SIZE>>; 3] {
        let header = format!(MENU_LINE_SIZE; "Time").unwrap();
        let time = get_current_time().clone();
        let time_since_2000 = time.year_since_1970.saturating_sub(30);
        let time_string = format!(MENU_LINE_SIZE; "{:02}/{:02}/{:02},{:02}:{:02}", 
            time.zero_indexed_month.saturating_add(1), time.zero_indexed_day.saturating_add(1), time_since_2000, time.hours, time.minutes).unwrap_or(format!(MENU_LINE_SIZE; " a ").unwrap());

        let cursor_string = match self.time_menu.time_index {
            0 => format!(MENU_LINE_SIZE; " ^             "),
            1 => format!(MENU_LINE_SIZE; "    ^          "),
            2 => format!(MENU_LINE_SIZE; "       ^       "),
            3 => format!(MENU_LINE_SIZE; "          ^    "),
            _ => format!(MENU_LINE_SIZE; "             ^ "),
        }
        .unwrap();

        [Some(header), Some(time_string), Some(cursor_string)]
    }
}
