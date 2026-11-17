use crate::{
    cross_task_data::{
        get_current_bat_amount, get_current_power_level, get_current_time, get_most_recent_epc,
        set_power_level, update_device_time,
    },
    rfid_task::EPC,
    sd_card_task::format_epc,
};
use core::fmt::Write;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Receiver, Sender},
};
use embedded_sdmmc::Timestamp;
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

    pub fn select_option(&self) {
        match self.session_choice {
            RestoreSessionOption::RestoreSession => {
                todo!("Compute from last session");
            },
            RestoreSessionOption::NewSession => {/* Do nothing */},
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
    // "Power Level"
    // changing power level
    power_level_digit: usize,
    power_digits: [u8; 2],
}

impl PowerMenu {
    pub fn new(mut power_level: u16) -> Self {
        let mut power_digits = [0u8; 2];
        for i in (0..2).rev() {
            power_digits[i] = (power_level % 10) as u8;
            power_level /= 10;
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

struct ReadTagMenu {
    tag_index: usize,
    // Recieves read tags from the SD card
    epc_tag_reciever: Receiver<'static, CriticalSectionRawMutex, Option<EPC>, 8>,
    // Sends a tag request to the SD card
    epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
}

impl ReadTagMenu {
    pub fn new(
        epc_tag_reciever: Receiver<'static, CriticalSectionRawMutex, Option<EPC>, 8>,
        epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
    ) -> Self {
        Self {
            tag_index: 0,
            epc_tag_reciever,
            epc_tag_request_sender,
        }
    }

    pub fn increase_index(&mut self) {
        self.tag_index += 1;
    }

    pub fn decrease_index(&mut self) {
        self.tag_index -= 1;
    }

    pub async fn get_rfid_tag(&self, offset: usize) -> Option<EPC> {
        self.epc_tag_request_sender
            .send(Some(self.tag_index + offset))
            .await;
        self.epc_tag_reciever.receive().await
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

    pub fn get_time(&self) -> String<16> {
        format!(
            16;
            "{:02}/{:02}/{:02},{:02}:{:02}",
            self.time_values[0],
            self.time_values[1],
            self.time_values[2],
            self.time_values[3],
            self.time_values[4],
        )
        .unwrap()
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

    // We only have state data on some states
    restore_session_menu: RestoreSessionMenu,
    power_menu: PowerMenu,
    tag_menu: ReadTagMenu,
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
    ReadTags,
    Temp,
    Time,
}

impl UIMenu {
    pub fn new(
        epc_tag_reciever: Receiver<'static, CriticalSectionRawMutex, Option<EPC>, 8>,
        epc_tag_request_sender: Sender<'static, CriticalSectionRawMutex, Option<usize>, 8>,
    ) -> Self {
        Self {
            // Start on the main screen
            active_menu_state: ActiveMenuState::Loading,
            // State data
            restore_session_menu: RestoreSessionMenu::new(epc_tag_request_sender),
            power_menu: PowerMenu::new(get_current_power_level()),
            tag_menu: ReadTagMenu::new(epc_tag_reciever, epc_tag_request_sender),
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
                self.restore_session_menu.epc_tag_request_sender.send(None).await;
                self.active_menu_state = ActiveMenuState::Main
            },
            ActiveMenuState::Main => self.active_menu_state = ActiveMenuState::Power,
            ActiveMenuState::Power => self.active_menu_state = ActiveMenuState::ReadTags,
            ActiveMenuState::ReadTags => self.active_menu_state = ActiveMenuState::Temp,
            ActiveMenuState::Temp => self.active_menu_state = ActiveMenuState::Time,
            ActiveMenuState::Time => self.active_menu_state = ActiveMenuState::Main,
        }
    }

    pub async fn get_display(&self) -> [Option<String<16>>; 3] {
        match self.active_menu_state {
            ActiveMenuState::Loading => self.get_loading_text(),
            ActiveMenuState::RestoreLastSession => self.get_restore_session_test(),
            ActiveMenuState::Main => self.get_main_menu_text().await,
            ActiveMenuState::Power => self.get_power_menu_text(),
            ActiveMenuState::ReadTags => self.get_tag_menu_text().await,
            ActiveMenuState::Temp => self.get_temp_menu_text(),
            ActiveMenuState::Time => self.get_time_menu_text().await,
        }
    }

    pub fn press_trigger_button(&mut self) {
        // Currently doesn't do anything
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

    pub async fn press_lock_button(&mut self) {
        match self.active_menu_state {
            ActiveMenuState::RestoreLastSession => {
                self.next().await;
            }
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
    pub fn get_loading_text(&self) -> [Option<String<16>>; 3] {
        let string_1 = format!(16; "Starting up...").unwrap();
        [None, Some(string_1), None]
    }

    pub fn get_restore_session_test(&self) -> [Option<String<16>>; 3] {
        let string_1 = format!(16; "Restore session?").unwrap();
        let string_2 = 
        if self.restore_session_menu.session_choice == RestoreSessionOption::RestoreSession {
            format!(16; "Yes <")
        } else {
            format!(16; "Yes")
        }.unwrap();
        let string_3 = 
        if self.restore_session_menu.session_choice == RestoreSessionOption::NewSession {
            format!(16; "New session <")
        } else {
            format!(16; "New session")
        }.unwrap();
        [Some(string_1), Some(string_2), Some(string_3)]
    }

    pub async fn get_main_menu_text(&self) -> [Option<String<16>>; 3] {
        let string_1 = format!(16; "{} bats read", get_current_bat_amount()).unwrap();
        let epc = get_most_recent_epc().await;
        let string_2 = match epc {
            Some(epc) => format_epc(&epc),
            None => format!(16; "no recent tags").unwrap(),
        };
        let string_3 = format!(16; "power: {}dB", get_current_power_level() / 100).unwrap();

        [Some(string_1), Some(string_2), Some(string_3)]
    }

    pub fn get_power_menu_text(&self) -> [Option<String<16>>; 3] {
        let header_string = format!(16; "Power Level").unwrap();
        let power_level_string = format!(16; "{:02}dB", get_current_power_level() / 100).unwrap();
        let cursor_string = match self.power_menu.power_level_digit {
            0 => format!(16; "^   "),
            _ => format!(16; " ^  "),
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

    pub async fn get_tag_menu_text(&self) -> [Option<String<16>>; 3] {
        let header = format!(16; "RFID Tags").unwrap();
        let epc_1_option = self.tag_menu.get_rfid_tag(0).await;
        let epc_2_option = self.tag_menu.get_rfid_tag(1).await;
        let mut epc_string_1 = heapless::String::<16>::new();
        let mut epc_string_2 = heapless::String::<16>::new();

        if let Some(epc_1) = epc_1_option {
            for idx in 0..usize::from(epc_1.bytes.len()) {
                write!(epc_string_1, "{:02X} ", epc_1.bytes[idx]).unwrap();
            }
        } else {
            write!(epc_string_1, "END").unwrap();
        }

        if let Some(epc_2) = epc_2_option {
            for idx in 0..usize::from(epc_2.bytes.len()) {
                write!(epc_string_2, "{:02X} ", epc_2.bytes[idx]).unwrap();
            }
        } else {
            write!(epc_string_2, "END").unwrap();
        }

        // We need a scrolling list of RFID tags
        [Some(header), Some(epc_string_1), Some(epc_string_2)]
    }

    pub fn get_temp_menu_text(&self) -> [Option<String<16>>; 3] {
        let header = format!(16; "Temperature").unwrap();
        let temperature = format!(16; "0.0F").unwrap();

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
            0 => format!(16; " ^             "),
            1 => format!(16; "    ^          "),
            2 => format!(16; "       ^       "),
            3 => format!(16; "          ^    "),
            _ => format!(16; "             ^ "),
        }
        .unwrap();

        [Some(header), Some(time_string), Some(cursor_string)]
    }
}
