use once_cell::sync::OnceCell;
use std::sync::{Arc, Mutex};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STDERR: Arc<Mutex<StandardStream>> =
        Arc::new(Mutex::new(StandardStream::stderr(ColorChoice::Always)));
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Info,
    Success,
    Warn,
    Error,
    Silent,
}

impl LogLevel {
    pub fn from_str(level: &str) -> Option<Self> {
        match level.to_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            "success" => Some(Self::Success),
            "silent" => Some(Self::Silent),
            _ => None,
        }
    }
}

static LOG_LEVEL: OnceCell<LogLevel> = OnceCell::new();

pub fn set_global_log_level(level: LogLevel) {
    LOG_LEVEL
        .set(level)
        .expect("Log level has already been set");
}

pub fn get_global_log_level() -> LogLevel {
    *LOG_LEVEL.get().unwrap_or(&LogLevel::Info) // Default to LogLevel::Info if not set
}

macro_rules! debug {
  ($($arg:tt)*) => {{
      if crate::logging::get_global_log_level() <= crate::logging::LogLevel::Debug {
          let mut stderr = crate::logging::STDERR.lock().unwrap();
          let _ = stderr.set_color(crate::logging::ColorSpec::new().set_fg(Some(Color::Magenta))); // Set color to magenta for debug
          let _ = writeln!(&mut *stderr, $($arg)*);
          let _ = stderr.reset();
      }
  }};
}

macro_rules! info {
  ($($arg:tt)*) => {{
      if get_global_log_level() <= LogLevel::Info {
          let mut stderr = StandardStream::stderr(ColorChoice::Always);
          // let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::)));
          let _ = writeln!(&mut stderr, $($arg)*);
          // let _ = stderr.reset();
      }
  }};
}

macro_rules! warn {
  ($($arg:tt)*) => {{
      if get_global_log_level() <= LogLevel::Warn {
          let mut stderr = STDERR.lock().unwrap();
          let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow))); // Set color to yellow for warning
          let _ = writeln!(&mut *stderr, $($arg)*);
          let _ = stderr.reset();
      }
  }};
}

// Reusing your existing error macro
macro_rules! error {
  ($($arg:tt)*) => {{
      if get_global_log_level() <= LogLevel::Error {
          let mut stderr = STDERR.lock().unwrap();
          let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red))); // Set color to red
          let _ = writeln!(&mut *stderr, $($arg)*);
          let _ = stderr.reset();
      }
  }};
}

macro_rules! success {
  ($($arg:tt)*) => {{
      if get_global_log_level() <= LogLevel::Success {
          let mut stderr = STDERR.lock().unwrap();
          let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Green))); // Set color to blue for success
          let _ = writeln!(&mut *stderr, $($arg)*);
          let _ = stderr.reset();
      }
  }};
}
