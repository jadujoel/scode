lazy_static::lazy_static! {
    pub static ref STDERR: std::sync::Arc<std::sync::Mutex<termcolor::StandardStream>> =
        std::sync::Arc::new(std::sync::Mutex::new(termcolor::StandardStream::stderr(termcolor::ColorChoice::Always)));
}
pub static LOG_LEVEL: once_cell::sync::OnceCell<crate::logging::LogLevel> =
    once_cell::sync::OnceCell::new();

pub fn set_loglevel(level: crate::logging::LogLevel) {
    LOG_LEVEL
        .set(level)
        .expect("Log level has already been set");
}

pub fn get_loglevel() -> crate::logging::LogLevel {
    *LOG_LEVEL.get().unwrap_or(&LogLevel::Info)
}

pub fn is_debug() -> bool {
    get_loglevel() <= LogLevel::Debug
}

pub fn is_perf() -> bool {
    get_loglevel() <= LogLevel::Perf
}

// pub fn is_info() -> bool {
//     get_loglevel() <= LogLevel::Info
// }

// pub fn is_warn() -> bool {
//     get_loglevel() <= LogLevel::Warn
// }

// pub fn is_error() -> bool {
//     get_loglevel() <= LogLevel::Error
// }

// pub fn is_success() -> bool {
//     get_loglevel() <= LogLevel::Success
// }

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Debug {
            let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Magenta)));
            let _ = writeln!(&mut stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[macro_export]
macro_rules! perf {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Perf {
            let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Cyan)));
            let _ = writeln!(&mut stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[macro_export]
macro_rules! time {
    ($label:expr, $block:block) => {{
        let _timer = if $crate::logging::is_perf() {
            Some($crate::logging::Timer::new($label))
        } else {
            None
        };
        // Execute the code block
        let result = $block;

        // _timer is dropped here if it was created, logging the elapsed time
        result // Return the result of the block execution, if needed
    }};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Info {
            let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Always);
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::White)));
            let _ = writeln!(&mut stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Warn {
            let mut stderr = $crate::logging::STDERR.lock().unwrap();
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Yellow))); // Set color to yellow for warning
            let _ = writeln!(&mut *stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Error {
            let mut stderr = $crate::logging::STDERR.lock().unwrap();
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Red))); // Set color to red
            let _ = writeln!(&mut *stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        use termcolor::WriteColor;
        use std::io::Write;
        if $crate::logging::get_loglevel() <= $crate::logging::LogLevel::Success {
            let mut stderr = $crate::logging::STDERR.lock().unwrap();
            let _ = stderr.set_color(termcolor::ColorSpec::new().set_fg(Some(termcolor::Color::Green))); // Set color to blue for success
            let _ = writeln!(&mut *stderr, $($arg)*);
            let _ = stderr.reset();
        }
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Perf,
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
            "perf" => Some(Self::Perf),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            "success" => Some(Self::Success),
            "silent" => Some(Self::Silent),
            _ => None,
        }
    }
}

pub fn log_progress(
    start: std::time::Instant,
    ns: usize,
    n: usize,
) {
    if get_loglevel() <= LogLevel::Info {
        let elapsed_time = start.elapsed().as_millis();
        let avg_time_per_sound = elapsed_time as f32 / ns as f32;
        let remaining_sounds = n - ns;
        let remaining_time = (remaining_sounds as f32 * avg_time_per_sound) as u64;
        let percentage = (ns as f32 / n as f32) * 100.0;
        print!(
            "Encoding {ns} of {n} ({percentage:.1}%) | ETA: {} seconds  \r",
            duration(u128::from(remaining_time))
        );
    }
}

pub fn duration(milliseconds: u128) -> String {
    if milliseconds < 1000 {
        return format!("{milliseconds}ms");
    }
    let minutes = milliseconds / 60000;
    let seconds = (milliseconds % 60000) / 1000;
    let remaining_ms = milliseconds % 1000;
    if minutes == 0 {
        if seconds == 0 {
            format!("{remaining_ms}ms")
        } else {
            format!("{seconds}s {remaining_ms}ms")
        }
    } else {
        format!("{minutes}m {seconds}s {remaining_ms}ms")
    }
}

pub fn duration_from_micros(micros: u128) -> String {
    if micros < 1000 {
        return format!("{micros}μs");
    }
    let milliseconds = micros / 1000;
    duration(milliseconds)
}

use std::fmt;
use std::time::Instant;

lazy_static::lazy_static! {
    static ref TIMINGS: std::sync::Mutex<Vec<(String, std::time::Duration)>> = std::sync::Mutex::new(Vec::new());
}

pub fn store_timing(label: String, duration: std::time::Duration) {
    TIMINGS.lock().unwrap().push((label, duration));
}

pub struct Timer<'a> {
    label: &'a str,
    start: Instant,
}

impl<'a> Timer<'a> {
    pub fn new(label: &'a str) -> Self {
        // debug!("{label}");
        Self {
            label,
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    // pub fn elapsed_micros(&self) -> u128 {
    //     self.start.elapsed().as_micros()
    // }
}

impl<'a> fmt::Display for Timer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", duration(self.elapsed_ms()))
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        // perf!("{} took {}", self.label, duration(self.elapsed_ms()));
        let duration = self.start.elapsed();
        store_timing(self.label.to_string(), duration);
    }
}

pub fn display_timings() {
    let timings: std::sync::MutexGuard<Vec<(String, std::time::Duration)>> = TIMINGS.lock().unwrap();
    let mut timings: Vec<_> = timings.iter().collect();
    timings.sort_by(|a, b| b.1.cmp(&a.1));

    perf!("Times:");
    let mut total = 0;
    for (label, taken) in timings {
        let micros = taken.as_micros();
        perf!("{} [{}]", duration_from_micros(micros), label);
        total += micros;
    }
    perf!("Total: {}", duration_from_micros(total));
}
pub struct TimingsDisplay;

impl Drop for TimingsDisplay {
    fn drop(&mut self) {
        display_timings();
    }
}
