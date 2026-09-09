use esp_hal::{spi::master::{Config, ConfigError, Spi}, time::Rate};

pub struct asdSDCard {

}

impl asdSDCard {
    pub fn new(spi_peripheral: esp_hal::peripherals::SPI2<'_>, speed: Rate, mosi: ) -> Result<(), ConfigError> {

        

        Self { }
    }
}