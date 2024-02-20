use std::time::Instant;

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

pub fn log_progress(ns: usize, num_sounds_to_encode: usize, start: Instant, log_level: LogLevel) {
    if log_level >= LogLevel::Info {
        let elapsed_time = start.elapsed().as_millis();
        let avg_time_per_sound = elapsed_time as f32 / ns as f32;
        let remaining_sounds = num_sounds_to_encode - ns;
        let remaining_time = (remaining_sounds as f32 * avg_time_per_sound) as u64;
        let percentage = (ns as f32 / num_sounds_to_encode as f32) * 100.0;
        println!(
            "Encoding {ns} of {num_sounds_to_encode} ({percentage:.1}%) | ETA: {} seconds  \r",
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
