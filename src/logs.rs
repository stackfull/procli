use log::*;

use crate::config::Logging;

#[derive(Debug)]
pub struct LogManager;

static LOGGER: LogManager = LogManager;

impl LogManager {
    pub fn configure(config: Logging) -> color_eyre::Result<()> {
        set_logger(&LOGGER)?;
        set_max_level(config.level);
        Ok(())
    }
}

impl Log for LogManager {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
