use log::{LevelFilter, Metadata, Record, SetLoggerError};

use crate::prelude::*;

static LOGGER: GlobalConsoleLogger = GlobalConsoleLogger;

pub fn initialize(level_filter: LevelFilter) -> core::result::Result<(), SetLoggerError> {
    log::set_logger(&LOGGER).map(|()| log::set_max_level(level_filter))
}

struct GlobalConsoleLogger;
impl log::Log for GlobalConsoleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
