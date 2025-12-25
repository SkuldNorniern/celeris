use log::{LevelFilter, Log, Metadata, Record};
use std::time::SystemTime;

pub struct SimpleLogger;

impl Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            // Check if this is a console log from JavaScript
            let is_console_log = record.target() == "js-console";
            
            if is_console_log {
                // Format console logs with a separator
                let separator = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";
                let level_str = match record.level() {
                    log::Level::Error => "ERROR",
                    log::Level::Warn => "WARN",
                    log::Level::Info => "INFO",
                    log::Level::Debug => "DEBUG",
                    log::Level::Trace => "TRACE",
                };
                
                println!("{}", separator);
                println!("[JS Console.{}] {}", level_str, record.args());
                println!("{}", separator);
            } else {
                // Regular log format
                let location = match (record.file(), record.line()) {
                    (Some(file), Some(line)) => format!("{}:{}", file, line),
                    (Some(file), None) => file.to_string(),
                    (None, _) => String::from("unknown location"),
                };

                println!(
                    "[{level}][{target}][{location}] {message}",
                    level = record.level(),
                    target = record.target(),
                    location = location,
                    message = record.args()
                );
            }
        }
    }

    fn flush(&self) {}
}

pub fn init(level: LevelFilter) -> Result<(), log::SetLoggerError> {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).map(|()| log::set_max_level(level))
}
