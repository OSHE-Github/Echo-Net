struct MainMenu {
    // Bat amount read this session
    // Last RFID read
    // Power Level
}

impl MainMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_text(&self) -> &'static str {
        ""
    }
}

struct PowerMenu {
    // "Power Level"
    // changing power level
}

impl PowerMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_text(&self) -> &'static str {
        ""
    }
}

struct ReadTagMenu {
    // "RFID Tags"
    // Scrolling tags
}

impl ReadTagMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_text(&self) -> &'static str {
        ""
    }
}

struct TempMenu {
    // "Temperature"
    // temperature
}

impl TempMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_text(&self) -> &'static str {
        ""
    }
}

struct TimeMenu {
    // "Time"
    // changeable time
}

impl TimeMenu {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_text(&self) -> &'static str {
        ""
    }
}

struct UIMenu {
    active_menu_state: ActiveMenuState,

    main_menu: MainMenu,
    power_menu: PowerMenu,
    tag_menu: ReadTagMenu,
    temp_menu: TempMenu,
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

    pub fn get_display(&self) -> &'static str {
        match self.active_menu_state {
            ActiveMenuState::Main => {
                self.main_menu.get_text()
            },
            ActiveMenuState::Power => {
                self.power_menu.get_text()
            },
            ActiveMenuState::ReadTags => {
                self.tag_menu.get_text()
            },
            ActiveMenuState::Temp => {
                self.temp_menu.get_text()
            },
            ActiveMenuState::Time => {
                self.time_menu.get_text()
            },
        }\
    }
}