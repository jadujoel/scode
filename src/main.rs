#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[macro_use]
extern crate lazy_static;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufRead, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

mod wave;
use wave::Data;

lazy_static! {
    static ref STDERR: Arc<Mutex<StandardStream>> =
        Arc::new(Mutex::new(StandardStream::stderr(ColorChoice::Always)));
}

macro_rules! debug {
    ($($arg:tt)*) => {{
        if get_global_log_level() <= LogLevel::Debug {
            let mut stderr = STDERR.lock().unwrap();
            let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Magenta))); // Set color to magenta for debug
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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SoundFileInfo {
    path: String,
    name: String,
    outfile: String,
    package: String,
    lang: String,
    output_path: String,
    bitrate: u32,
    num_samples: usize,
    sample_rate: u32,
    modification_date: String,
}

fn write_hashmap_to_json_pretty(file_infos: &HashMap<String, SoundFileInfo>) -> io::Result<()> {
    // Ensure the cache directory exists
    let cache_dir = Path::new(".cache");
    std::fs::create_dir_all(cache_dir)?;

    // Create and open the file
    let file_path = cache_dir.join("info.json");
    let file = File::create(file_path)?;

    // Serialize and write data as pretty JSON
    serde_json::to_writer_pretty(file, &file_infos)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
}

fn write_sound_info_to_json(output_file: &str, finfo: &[SoundFileInfo]) -> io::Result<()> {
    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    let mut groups: HashMap<String, Vec<&SoundFileInfo>> = HashMap::new();

    // Group the sound files by their package
    for info in finfo {
        groups.entry(info.package.clone()).or_default().push(info);
    }

    writeln!(writer, "{{")?;
    for (index, package) in groups.iter().enumerate() {
        write!(writer, "\"{}\": [", package.0)?;
        for (index, info) in package.1.iter().enumerate() {
            if info.lang == "none" {
                // If lang is "none", skip the lang field
                write!(
                    writer,
                    "\n  [\"{}\", \"{}\", {}]",
                    info.name,
                    info.outfile.replace(".webm", ""),
                    info.num_samples,
                )?;
            } else {
                // If lang is not "none", include it in the JSON
                write!(
                    writer,
                    "\n  [\"{}\", \"{}\", {}, \"{}\"]",
                    info.name,
                    info.outfile.replace(".webm", ""),
                    info.num_samples,
                    info.lang,
                )?;
            }

            // Comma between items, not after the last item
            if index < package.1.len() - 1 {
                write!(writer, ", ")?;
            } else {
                write!(writer, "\n]")?;
            }
        }
        // Handle commas between objects
        writeln!(
            writer,
            "{}",
            if index < groups.len() - 1 { "," } else { "" }
        )?;
    }
    writeln!(writer, "}}")?;

    Ok(())
}

static LOG_LEVEL: OnceCell<LogLevel> = OnceCell::new();

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum LogLevel {
    Debug,
    Info,
    Success,
    Warn,
    Error,
    Silent,
}

impl LogLevel {
    fn from_str(level: &str) -> Option<Self> {
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

fn set_global_log_level(level: LogLevel) {
    LOG_LEVEL
        .set(level)
        .expect("Log level has already been set");
}

fn get_global_log_level() -> LogLevel {
    *LOG_LEVEL.get().unwrap_or(&LogLevel::Info) // Default to LogLevel::Log if not set
}

#[allow(clippy::struct_excessive_bools)]
struct Args {
    indir: String,
    outdir: String,
    ffmpeg: String,
    packages: Vec<String>,
    include_mp4: bool,
    bitrate: u32,
    yes: bool,
    skip_cache: bool,
    loglevel: LogLevel,
    help: bool,
}

#[derive(Debug, Clone)]
struct FilePath {
    path_buf: PathBuf,
    package: String,
    package_path: PathBuf,
    lossy: String,
}

fn parse_args(args: &[String]) -> Args {
    // Initialize with default values
    let mut indir = String::from("packages");
    let mut outdir = String::from("encoded");
    let mut ffmpeg = String::from("ffmpeg");
    let mut packages: Vec<String> = Vec::new();
    let mut include_mp4 = true;
    let mut yes = false;
    let mut bitrate = 96;
    let mut skip_cache = false;
    let mut loglevel = LogLevel::Info;
    let mut help = false;

    for arg in args.iter().skip(1) {
        match arg {
            a if a == "-h" || a == "--help" => {
                help = true;
            }
            a if a.starts_with("--indir=") => {
                indir = a["--indir=".len()..].trim_matches('"').to_string();
            }
            a if a.starts_with("--outdir=") => {
                outdir = a["--outdir=".len()..].trim_matches('"').to_string();
            }
            a if a.starts_with("--ffmpeg=") => {
                ffmpeg = a["--ffmpeg=".len()..].trim_matches('"').to_string();
            }
            a if a.starts_with("--packages=") => {
                packages = a["--packages=".len()..]
                    .trim_matches('"')
                    .split(',')
                    .map(String::from)
                    .collect();
            }
            a if a == "--no-mp4" => include_mp4 = false,
            a if a == "-y" || a == "--yes" => yes = true,
            a if a.starts_with("--bitrate=") => {
                bitrate = a["--bitrate=".len()..]
                    .trim_matches('"')
                    .parse()
                    .unwrap_or(96);
            }
            a if a.starts_with("--skip-cache") => skip_cache = true,
            a if a.starts_with("--loglevel=") => {
                if let Some(level) = LogLevel::from_str(&a["--loglevel=".len()..]) {
                    loglevel = level;
                }
            }
            _ => {}
        }
    }

    Args {
        indir,
        outdir,
        ffmpeg,
        packages,
        include_mp4,
        bitrate,
        yes,
        skip_cache,
        loglevel,
        help,
    }
}

// Function to get the modification date as a String
fn get_modification_date_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let metadata = fs::metadata(path)?;
    let modified_time = metadata.modified()?;
    // Convert SystemTime to a formatted string or a simple representation
    // Here, we convert it to UNIX timestamp for simplicity
    let datetime: DateTime<Utc> = modified_time.into();
    Ok(datetime.to_rfc3339())
}

// Function to save a HashMap of SoundFileInfo to disk
fn save_cache(sound_info: &HashMap<String, SoundFileInfo>, file_path: &Path) -> io::Result<()> {
    let encoded: Vec<u8> = bincode::serialize(sound_info)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let mut file = File::create(file_path)?;
    file.write_all(&encoded)?;
    Ok(())
}

// Function to load a HashMap of SoundFileInfo from disk
fn load_cache(file_path: &Path) -> io::Result<HashMap<String, SoundFileInfo>> {
    let mut file = File::open(file_path)?;
    let mut encoded = Vec::new();
    file.read_to_end(&mut encoded)?;
    let sound_info: HashMap<String, SoundFileInfo> = bincode::deserialize(&encoded)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    Ok(sound_info)
}

fn main() -> io::Result<()> {
    let start_time = Instant::now(); // Record start time
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(args.as_ref());
    if parsed.help {
        info!(
            "Usage: {} --indir=<directory> [--outdir=<output-file>] [--ffmpeg=<path-to-ffmpeg>] [--packages=<package1,package2,...>] [--no-mp4] [--bitrate=<bitrate>] [--skip-cache] [--loglevel=<debug|info|warn|error|silent>]",
            args[0]
        );
        return Ok(());
    }
    set_global_log_level(parsed.loglevel);

    if parsed.indir.is_empty() {
        debug!(
            "Usage: {} --indir=<directory> [--outdir=<output-file>]",
            args[0]
        );
        return Ok(());
    }
    if parsed.packages.is_empty() {
        info!("Encoding all packages");
    } else {
        info!("Encoding packages: {:?}", parsed.packages);
    };
    debug!("Parsed args in {} ms", start_time.elapsed().as_millis());
    run(&parsed)?;
    let elapsed_time = start_time.elapsed();
    let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
    let elapsed = format_duration(u128::from(elapsed_ms));
    success!("Done in {elapsed}");
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run(parsed: &Args) -> io::Result<()> {
    let output_bitrate = parsed.bitrate;
    let now = Instant::now();
    let wav_files: Vec<PathBuf> = WalkDir::new(&parsed.indir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            if entry.path().extension()?.to_str()? == "wav" {
                Some(entry.into_path())
            } else {
                None
            }
        })
        .collect();

    let num_sounds: u128 = wav_files.len() as u128 + 1;
    debug!(
        "Found {} wav files in {} ms, {} microseconds per sound",
        wav_files.len(),
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let cache_path = Path::new(".cache");
    let cached = if cache_path.exists() && !parsed.skip_cache {
        let res = load_cache(Path::new(".cache/info.bin")).unwrap_or_default();
        debug!(
            "Loaded sound info from disk in {} ms, {} microseconds per sound",
            now.elapsed().as_millis(),
            now.elapsed().as_micros() / num_sounds
        );
        res
    } else {
        debug!("No cache found, creating new cache");
        fs::create_dir_all(cache_path)?;
        HashMap::new()
    };

    let now = Instant::now();
    let paths: Vec<FilePath> = wav_files
        .par_iter()
        .filter_map(|path_buf| {
            let path = path_buf.as_path();
            let lossy = path.to_string_lossy().to_string();
            let package = lossy
                .split("packages")
                .nth(1)
                .unwrap_or("_")
                .split('/')
                .nth(1)
                .unwrap_or("_")
                .to_string();

            let package_path =
                lossy.split("packages").next().unwrap().to_string() + "packages/" + &package;
            let package_path = Path::new(package_path.as_str());

            if !parsed.packages.is_empty() && !parsed.packages.contains(&package) {
                return None;
            }

            Some(FilePath {
                path_buf: path_buf.clone(),
                package,
                package_path: package_path.to_path_buf(),
                lossy,
            })
        })
        .collect();
    debug!(
        "Filtered file paths in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let finfo: Vec<Result<SoundFileInfo, io::Error>> = paths
        .clone()
        .into_iter()
        .map(|file_path| {
            let path_buf = file_path.path_buf.clone();
            let path = file_path.path_buf.as_path();
            let lossy = file_path.lossy.clone();
            let package = file_path.package.clone();
            let package_path = file_path.package_path.clone();

            // if the file has not been modified since the last time we hashed it
            // we use the cached info
            // downside is that if .lang or .bitrates files have been added or removed or changed
            // we won't know about it
            let modification_date = get_modification_date_string(path).unwrap_or_default();
            if let Some(cached_info) = cached.get(&lossy) {
                if modification_date == cached_info.modification_date {
                    return Ok(cached_info.clone());
                }
            }

            let mut file = match File::open(path_buf) {
                Ok(file) => file,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error opening file: {lossy} {e:?}"),
                    ));
                }
            };
            let mut buffer = Vec::new();

            // this bit takes the longest time to run
            // we're using it to hash the entire file
            match file.read_to_end(&mut buffer).ok() {
                Some(_) => (),
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Error reading file: {lossy}"),
                    ));
                }
            }

            let mut hasher = DefaultHasher::new();
            buffer.hash(&mut hasher);
            let hash = hasher.finish().to_string();

            let wave_data = match Data::from_buffer(&buffer) {
                Ok(wave_data) => wave_data,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Error reading WAV data from {lossy}: {e:?}"),
                    ));
                }
            };

            // let lang_start_time = Instant::now();
            // find a language file in the parent directory
            // for example src/packages/localisationprototype/sounds/en/.lang
            // that contains the language of the sound file
            // eg "english" or "french"
            let parent = path.parent().unwrap_or(Path::new(""));
            let lang_path = parent.join(".lang");
            let lang = if lang_path.is_file() {
                let mut lang_file = File::open(lang_path).unwrap();
                let mut lang = String::new();
                lang_file.read_to_string(&mut lang).unwrap();
                lang.trim().to_string()
            } else {
                "none".to_string()
            };
            // // lang_time += lang_start_time.elapsed().as_micros() as f32;

            // let strops_start_time = Instant::now();
            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");

            let num_channels = wave_data.format.num_channels;
            let outfile = format!("{output_bitrate}kb.{num_channels}ch.{hash}.webm");

            let mut output_path = PathBuf::from(&parsed.outdir);
            output_path.push(outfile.clone());

            let name = filename.to_string().replace(".wav", "");
            // // strops_time += strops_start_time.elapsed().as_micros() as f32;

            // let bitrates_start_time = Instant::now();
            // update the bitrate if theres a bitrates file with the sound name in it
            let bitrates_path = package_path.join(".bitrates");
            let mut bitrate = output_bitrate;

            // Check if the bitrates file exists and is indeed a file
            if bitrates_path.is_file() {
                // Attempt to open the bitrates file, directly returning an Err variant of Result if it fails
                let bitrates_file = File::open(&bitrates_path);
                let bitrates = std::io::BufReader::new(bitrates_file.unwrap());

                // Iterate over each line, trimming whitespace and skipping empty lines
                for line in bitrates
                    .lines()
                    .map_while(Result::ok)
                    .map(|line| line.trim_end().to_string())
                    .filter(|line| !line.is_empty())
                {
                    let mut parts = line.split_whitespace();
                    let sound_name = parts.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "Missing sound name in bitrates file")
                    });
                    let bitrate_str = parts.next().ok_or_else(|| {
                        io::Error::new(io::ErrorKind::Other, "Missing bitrate in bitrates file")
                    });
                    if sound_name.is_err() || bitrate_str.is_err() {
                        continue;
                    }
                    // Process the sound_name and bitrate_str...
                    if sound_name.unwrap_or("none") == name {
                        bitrate = bitrate_str.unwrap_or("96").parse().unwrap_or(bitrate);
                    }
                }
            }
            // // bitrate_time += bitrates_start_time.elapsed().as_micros() as f32;

            Ok(SoundFileInfo {
                path: lossy.to_string(),
                name,
                outfile,
                package,
                lang,
                output_path: output_path.to_string_lossy().into_owned(),
                bitrate,
                sample_rate: wave_data.format.sample_rate,
                num_samples: wave_data.num_samples,
                modification_date,
            })
        })
        .collect();

    debug!(
        "Created sound info in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let had_error = finfo.iter().any(Result::is_err);

    debug!(
        "Checked for errors in sound info in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let mut did_reencode_source_files = false;
    if had_error {
        let files_that_can_be_fixed: Vec<String> = finfo
            .par_iter()
            .enumerate()
            .filter_map(|(i, result)| {
                if let Err(e) = result {
                    let infile = paths[i].lossy.clone();
                    if e.kind() == io::ErrorKind::InvalidInput {
                        return Some(infile);
                    }
                }
                None
            })
            .collect();
        warn!("The following files are not using pcm format:");
        for file in &files_that_can_be_fixed {
            warn!("  {}", file);
        }
        if !parsed.yes {
            loop {
                success!("Do you want to reencode source the files? (y/n)");
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim() == "y" {
                    break;
                }
                if input.trim() == "n" {
                    error!("Exiting");
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "User cancelled reencoding of source file",
                    ));
                }
            }
        }
        for (i, result) in finfo.iter().enumerate() {
            if let Err(e) = result {
                let infile = paths[i].lossy.clone();
                let outfile = infile.replace(".wav", ".pcm.wav");
                if e.kind() == io::ErrorKind::InvalidInput {
                    info!("Converting file: {} to use pcm format", infile);
                    let output = Command::new(parsed.ffmpeg.clone())
                        .arg("-i")
                        .arg(&infile)
                        .arg("-ar")
                        .arg("48000")
                        .arg("-c:a") // Use "-c:a" to specify the audio codec
                        .arg("pcm_s16le") // Set the codec to pcm_s16le
                        .arg(&outfile)
                        .arg("-y")
                        .output()?;

                    // Handle command execution error
                    if !output.status.success() {
                        let error = String::from_utf8_lossy(&output.stderr);
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            error.to_string(),
                        ));
                    }
                    fs::remove_file(&infile)?;
                    fs::rename(&outfile, &infile)?;
                    did_reencode_source_files = true;
                }
            }
        }
        if did_reencode_source_files {
            info!("Had to reencode some source files, rerunning the program to recheck the source files");
            return run(parsed);
        }
        let files_that_cannot_be_fixed: Vec<String> = finfo
            .par_iter()
            .enumerate()
            .filter_map(|(i, result)| {
                if let Err(e) = result {
                    let infile = paths[i].lossy.clone();
                    if e.kind() != io::ErrorKind::InvalidInput {
                        return Some(infile);
                    }
                }
                None
            })
            .collect();
        error!("The following files cannot be fixed:");
        for file in &files_that_cannot_be_fixed {
            error!("  {}", file);
        }
        for result in &finfo {
            if let Err(e) = result {
                error!("{e}");
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }

    debug!(
        "Checked for errors in sound info in {} ms",
        now.elapsed().as_millis()
    );
    if had_error {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to read",
        ));
    }

    let now = Instant::now();
    let finfo = finfo
        .into_iter()
        .filter_map(Result::ok)
        .collect::<Vec<SoundFileInfo>>();

    info!("Found {} source files", finfo.len());
    debug!(
        "Filtered sound info in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let sounds_to_convert: Vec<&SoundFileInfo> = finfo
        .iter()
        .filter(|info| info.sample_rate != 48000)
        .collect();

    debug!(
        "Checked sample rates in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    if !sounds_to_convert.is_empty() {
        loop {
            warn!("The following files have a sample rate other than 48 kHz:");
            for info in &sounds_to_convert {
                warn!("  {}: {}", info.path, info.sample_rate);
            }
            if parsed.yes {
                break;
            }
            success!("Do you want to convert them to 48 kHz? (y/n)");
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() == "y" {
                break;
            }
            if input.trim() == "n" {
                error!("Exiting");
                // return error
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "User cancelled source file conversion to 48kHz",
                ));
            }
        }
    }
    debug!(
        "Checked with user about sample rates in {} ms",
        now.elapsed().as_millis()
    );

    let now = Instant::now();
    // Use a combination of `map` and `collect` to handle errors
    let results: Result<Vec<_>, io::Error> = sounds_to_convert
        .par_iter()
        .map(|info| {
            info!("Converting file: {}", info.path);
            let converted = info.path.replace("wav", "48000.wav");
            let output = Command::new(parsed.ffmpeg.clone())
                .arg("-i")
                .arg(&info.path)
                .arg("-ar")
                .arg("48000")
                .arg(&converted)
                .arg("-y")
                .output()?;

            // Handle command execution error
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(io::Error::new(io::ErrorKind::Other, error.to_string()));
            }

            fs::remove_file(&info.path)?;
            fs::rename(&converted, &info.path)?;
            Ok(())
        })
        .collect();

    if !sounds_to_convert.is_empty() {
        // Handle or propagate the result of the entire operation
        match results {
            Ok(_) => debug!("All files converted successfully."),
            Err(e) => return Err(e),
        }
        debug!(
            "Converted sample rates in {} ms, {} microseconds per sound",
            now.elapsed().as_millis(),
            now.elapsed().as_micros() / num_sounds
        );
        debug!("Since the sample rates were converted, the program will now rerun to recheck the source files");
        return run(parsed);
    }

    debug!(
        "Checked sample rates in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let output_path = Path::new(parsed.outdir.as_str()).join(".info.json");
    if let Some(parent_dir) = output_path.parent() {
        if !parent_dir.exists() {
            fs::create_dir_all(parent_dir)?;
        }
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to create output directory",
        ));
    }
    if let Some(output_path) = output_path.to_str() {
        // write_sound_info_to_json_by_package(output_path, &finfo)?;
        write_sound_info_to_json(output_path, &finfo)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to write sound info to JSON",
        ));
    }
    debug!(
        "Wrote sound info to JSON in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    if parsed.loglevel == LogLevel::Debug {
        print_langs(&finfo);
    }

    let now = Instant::now();
    let num_sounds_encoded = Arc::new(Mutex::new(0));
    let sounds_that_needs_encoding: Vec<&SoundFileInfo> = if parsed.packages.is_empty() {
        finfo
            .par_iter()
            .filter(|info| !Path::new(&info.output_path).exists())
            .collect()
    } else {
        finfo
            .par_iter()
            .filter(|info| {
                !Path::new(&info.output_path).exists() && parsed.packages.contains(&info.package)
            })
            .collect()
    };
    debug!(
        "Checked for sounds that need encoding in {} ms",
        now.elapsed().as_millis()
    );

    let now = Instant::now();
    let num_sounds_to_encode = sounds_that_needs_encoding.len();
    let results: Vec<Result<(), io::Error>> = sounds_that_needs_encoding
        .par_iter()
        .filter_map(|info| {
            if Path::new(&info.output_path).exists()
                || !parsed.packages.is_empty() && !parsed.packages.contains(&info.package)
            {
                return None;
            }
            *num_sounds_encoded.lock().unwrap() += 1;
            let ns = *num_sounds_encoded.lock().unwrap();
            let percentage = (ns as f32 / num_sounds_to_encode as f32) * 100.0;
            let elapsed_time = now.elapsed().as_secs();
            let avg_time_per_sound = elapsed_time as f32 / ns as f32;
            let remaining_sounds = num_sounds_to_encode - ns;
            let remaining_time = (remaining_sounds as f32 * avg_time_per_sound) as u64;
            if parsed.loglevel >= LogLevel::Info {
                print!("Encoding {ns} of {num_sounds_to_encode} ({percentage:.1}%) | ETA: {remaining_time} seconds  \r");
                io::stdout().flush().unwrap();
            }
            Some(run_ffmpeg(&parsed.ffmpeg, info, parsed.include_mp4))
        })
        .collect();
    if !results.is_empty() {
        info!(
            "Encoding {num_sounds_to_encode} of {num_sounds_to_encode} (100%) | ETA: 0 seconds  \r"
        );
    }
    debug!(
        "Encoded sounds in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let success = results.iter().all(Result::is_ok); // && *success.lock().unwrap();
    if !success {
        for (i, result) in results.iter().enumerate() {
            if let Err(e) = result {
                let infile = finfo[i].path.clone();
                let outfile = finfo[i].output_path.clone();
                error!("Error encoding sound file: {infile:?} to {outfile:?} {e}");
            }
        }
        error!("Encoding Failure");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }
    debug!(
        "Checked encoding results in {} ms",
        now.elapsed().as_millis()
    );

    let now = Instant::now();
    // Convert the vector to a HashMap
    let info_map: HashMap<String, SoundFileInfo> = finfo
        .into_iter()
        .map(|info| (info.path.clone(), info))
        .collect();
    save_cache(&info_map, Path::new(".cache/info.bin"))?;
    debug!(
        "Saved sound info to disk in {} ms",
        now.elapsed().as_millis(),
    );

    if parsed.loglevel == LogLevel::Debug {
        let now = Instant::now();
        write_hashmap_to_json_pretty(&info_map)?;
        debug!(
            "Wrote sound debug info to JSON in {} ms",
            now.elapsed().as_millis(),
        );
    }

    if !success {
        error!("Encoding Failure");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }

    Ok(())
}

fn format_duration(milliseconds: u128) -> String {
    if milliseconds < 1000 {
        return format!("{milliseconds}ms");
    }

    let minutes = milliseconds / 60000;
    let seconds = (milliseconds % 60000) / 1000;
    let remaining_ms = milliseconds % 1000;

    if minutes == 0 {
        format!("{seconds}s {remaining_ms}ms")
    } else {
        format!("{minutes}m {seconds}s {remaining_ms}ms")
    }
}

fn run_ffmpeg(ffmpeg: &str, info: &SoundFileInfo, include_mp4: bool) -> io::Result<()> {
    // canonicalize the input and output paths
    let in_path = Path::new(&info.path).canonicalize()?;
    let out_path = Path::new(&info.output_path);
    // println!("Processing {}", info.path);

    if let Some(out_dir) = out_path.parent() {
        if !out_dir.exists() {
            // create the output directory if it doesn't exist
            fs::create_dir_all(out_dir)?;
        }
    }

    let infile = info.path.clone();
    let outfile = info.output_path.clone();
    let output = Command::new(ffmpeg)
        .arg("-i")
        .arg(infile)
        .arg("-b:a")
        .arg(info.bitrate.to_string() + "k")
        .arg("-c:a")
        .arg("libopus")
        .arg("-ar")
        .arg("48000")
        // .arg("-af")
        // .arg("aresample=resampler=soxr")
        .arg("-map_metadata")
        .arg("-1") // This tells ffmpeg to not copy any metadata from the input
        .arg("-y")
        .arg(outfile)
        .output()?;

    if !output.status.success() {
        error!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "ffmpeg execution failed",
        ));
    }

    if !include_mp4 {
        return Ok(());
    }

    let outfile = info.output_path.replace("webm", "mp4");
    // write the mp4 file
    let output = Command::new(ffmpeg)
        .arg("-i")
        .arg(in_path.to_str().unwrap())
        .arg("-ar")
        .arg("48000")
        .arg("-movflags")
        .arg("faststart")
        .arg("-b:a")
        .arg(info.bitrate.to_string() + "k")
        .arg("-c:a")
        .arg("aac")
        .arg("-map_metadata")
        .arg("-1") // This tells ffmpeg to not copy any metadata from the input
        .arg(outfile)
        .arg("-y")
        .output()?;

    if !output.status.success() {
        error!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "ffmpeg execution failed",
        ));
    }
    Ok(())
}

fn print_langs(sound_info: &[SoundFileInfo]) {
    let mut langs: Vec<String> = sound_info.iter().map(|info| info.lang.clone()).collect();
    langs.sort();
    langs.dedup();
    debug!("Languages: {langs:?}");
}
