use colored::*;
use log::{Level, LevelFilter, Log, Metadata, Record};
use time::{OffsetDateTime, format_description::FormatItem};

const TIMESTAMP_FORMAT_OFFSET: &[FormatItem] = time::macros::format_description!(
  "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3][offset_hour sign:mandatory]:[offset_minute]"
);

struct SimpleLogger {
  level: LevelFilter,
}

impl Log for SimpleLogger {
  fn enabled(&self, metadata: &Metadata) -> bool { metadata.level().to_level_filter() <= self.level }

  fn log(&self, record: &Record) {
    if self.enabled(record.metadata()) {

      let Ok(Ok(timestamp)) = OffsetDateTime::now_local().map(|t|t.format(TIMESTAMP_FORMAT_OFFSET)) else {
        eprintln!("Failed to get local time");
        return;
      };

      let timestamp = format!("{timestamp:<29}").bright_black().to_string();

      let level_string = match record.level() {
        Level::Error => format!("{:<5}", record.level().to_string()).red().to_string(),
        Level::Warn => format!("{:<5}", record.level().to_string()).yellow().to_string(),
        Level::Info => format!("{:<5}", record.level().to_string()).cyan().to_string(),
        Level::Debug => format!("{:<5}", record.level().to_string()).purple().to_string(),
        Level::Trace => format!("{:<5}", record.level().to_string()).normal().to_string(),
      };

      let target = if record.target().is_empty() {
        record.module_path().unwrap_or_default()
      } else {
        record.target()
      };

      println!("{} {} [{}] {}", timestamp, level_string, target, record.args());
    }
  }

  fn flush(&self) {}
}

pub fn install_logger(verbose: bool) -> bool {
  let logger = SimpleLogger {
    level: if cfg!(debug_assertions) {
      if verbose {
        LevelFilter::Trace
      } else {
        LevelFilter::Debug
      }
    } else if verbose { LevelFilter::Debug } else { LevelFilter::Info },
  };

  log::set_max_level(logger.level);
  log::set_boxed_logger(Box::new(logger)).is_ok()
}
