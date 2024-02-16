#[macro_use]
extern crate lazy_static;

use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufRead, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

mod wave;
use wave::WaveData;

// for gzipping the json file
use flate2::write::GzEncoder;
use flate2::Compression;
// use humansize::{format_size, DECIMAL};
use serde_json::{Error, Value};

lazy_static! {
    static ref STDERR: Arc<Mutex<StandardStream>> =
        Arc::new(Mutex::new(StandardStream::stderr(ColorChoice::Always)));
}

macro_rules! eprintln {
    ($($arg:tt)*) => {{
        let mut stderr = STDERR.lock().unwrap();
        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red))); // Set color to red
        let _ = writeln!(&mut *stderr, $($arg)*); // Write the message
        let _ = stderr.reset(); // Reset color to default
    }};
}

// Example:
#[derive(Serialize, Deserialize, Debug)]
struct SoundFileInfo {
    path: String,
    name: String,
    outfile: String,
    hash: String,
    package: String,
    lang: String,
    // is_cached: bool,
    // cached_path: String,
    output_path: String,
    subdir: String,
    bitrate: u32,
    num_samples: usize,
    duration: f64,
    audio_format: String,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

fn write_sound_info_to_json(
    output_file: &str,
    sound_files_info: &[SoundFileInfo],
) -> io::Result<()> {
    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    let mut groups: HashMap<String, Vec<&SoundFileInfo>> = HashMap::new();

    // Group the sound files by their package
    for info in sound_files_info {
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

fn write_sound_info_to_json_by_package(
    output_file: &str,
    sound_files_info: &[SoundFileInfo],
) -> io::Result<()> {
    println!("Writing sound info to {}", output_file);

    let mut grouped_by_package: HashMap<String, Vec<&SoundFileInfo>> = HashMap::new();

    // Group the sound files by their package
    for info in sound_files_info {
        grouped_by_package
            .entry(info.package.clone())
            .or_default()
            .push(info);
    }

    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "{{")?;

    let mut package_iter = grouped_by_package.iter().peekable();
    while let Some((package, infos)) = package_iter.next() {
        write!(writer, "\"{}\": [", package)?;

        for (index, info) in infos.iter().enumerate() {
            // Write sound information without the package
            write!(
                writer,
                "\n  {{\"lang\": \"{}\", \"nc\": {}, \"br\": {}, \"name\": \"{}\", \"of\": \"{}\", \"hash\": {}, \"ns\": {}, \"dur\": {}}}",
                info.lang,
                info.num_channels,
                info.bitrate,
                info.name,
                info.outfile,
                info.hash,
                info.num_samples,
                info.duration
            )?;

            // Comma between items, not after the last item
            if index < infos.len() - 1 {
                write!(writer, ", ")?;
            }
        }

        write!(writer, "  ]")?;

        // Comma between packages, not after the last package
        if package_iter.peek().is_some() {
            writeln!(writer, ",")?;
        } else {
            writeln!(writer, "")?;
        }
    }

    writeln!(writer, "}}")?;

    Ok(())
}

struct Args {
    indir: String,
    outdir: String,
    ffmpeg: String,
    packages: Vec<String>,
    include_mp4: bool,
    yes: bool,
}

fn parse_args(args: &[String]) -> Args {
    let mut indir = String::from("packages");
    let mut outdir = String::from("encoded");
    let mut packages: Vec<String> = Vec::new();
    let mut include_mp4 = true;
    let mut yes = false;
    let mut ffmpeg = String::from("ffmpeg");

    for arg in args.iter().skip(1) {
        if arg.starts_with("--indir=") {
            indir = arg
                .trim_start_matches("--indir=")
                .trim_matches('"')
                .to_string();
        } else if arg.starts_with("--outdir=") {
            outdir = arg
                .trim_start_matches("--outdir=")
                .trim_matches('"')
                .to_string();
        } else if arg.starts_with("--packages=") {
            packages = arg
                .trim_start_matches("--packages=")
                .trim_matches('"')
                .split(',')
                .map(std::string::ToString::to_string)
                .collect();
        } else if arg.starts_with("--include-mp4") {
            include_mp4 = true;
        } else if arg.starts_with("--no-include-mp4") {
            include_mp4 = false;
        } else if (arg == "-y") || (arg == "--yes") {
            yes = true;
        } else if arg.starts_with("--ffmpeg=") {
            ffmpeg = arg
                .trim_start_matches("--ffmpeg=")
                .trim_matches('"')
                .to_string();
        }
    }

    Args {
        indir,
        outdir,
        ffmpeg,
        packages,
        include_mp4,
        yes,
    }
}

#[allow(clippy::too_many_lines)]
fn main() -> io::Result<()> {
    let start_time = Instant::now(); // Record start time
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(args.as_ref());

    if parsed.indir.is_empty() {
        println!(
            "Usage: {} --indir=<directory> [--outdir=<output-file>]",
            args[0]
        );
        return Ok(());
    }

    let output_bitrate = 96;

    // let mut cached_file: File;
    // let cached_path = Path::new(".cache/sounds.bin");
    // let mut encoded = Vec::new();

    // let mut cached: Vec<SoundFileInfo>;

    // if let Some(parent_dir) = cached_path.parent() {
    //     fs::create_dir_all(parent_dir)?;
    //     cached = Vec::new();
    // } else {
    //     return Err(io::Error::new(
    //         io::ErrorKind::Other,
    //         "Failed to create output directory",
    //     ));
    // }
    // if Path::new(cached_path).is_file() {
    //     println!("Cache exists!");
    //     cached_file = File::open(cached_path)?;
    //     cached_file.read_to_end(&mut encoded)?;
    //     cached = bincode::deserialize(&encoded).unwrap_or(Vec::new());
    // } else {
    //     println!("No Cache found.");
    // }
    // println!("{:#?}", cached);

    // Compile a regular expression to match filenames ending with .wav
    let wav_regex = Regex::new(r"\.wav$").expect("Invalid regex");

    // Initialize a Mutex-wrapped HashSet to store encountered hashes
    let hash_set: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    let success = Arc::new(Mutex::new(true)); // Initialize success as true

    // Walk the directory tree and calculate hash for each sound file
    let paths = WalkDir::new(&parsed.indir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.file_type().is_file() {
                return None;
            }
            let name = entry.file_name().to_str()?;
            if wav_regex.is_match(name) {
                return Some(entry.path().to_path_buf());
            }
            None
        })
        .collect::<Vec<PathBuf>>();

    let sound_files_info: Vec<SoundFileInfo> = paths
        .par_iter()
        .filter_map(|path_buf| {
            // Normalize the path to resolve any relative components
            // let path = fs::canonicalize(path_buf).ok()?;
            let path = path_buf.as_path();
            let mut file = File::open(path_buf).ok()?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).ok()?;
            let mut hasher = DefaultHasher::new();
            buffer.hash(&mut hasher);
            let hash = hasher.finish().to_string();
            // Check if hash already exists in the HashSet
            let mut locked_hash_set = match hash_set.lock() {
                Ok(hash_set) => hash_set,
                Err(poisoned) => {
                    eprintln!("Mutex poisoned: {poisoned}");
                    *success.lock().expect("Mutex poisoned") = false; // Set success to false if there's an error
                    return None;
                }
            };

            if !locked_hash_set.contains(&hash) {
                locked_hash_set.insert(hash.clone());
            }

            let wave_data = match WaveData::from_buffer(&buffer) {
                Ok(wave_data) => wave_data,
                Err(e) => {
                    eprintln!(
                        "Error reading WAV data from {}: {}",
                        path.to_string_lossy(),
                        e
                    );
                    *success.lock().expect("Mutex poisoned") = false;
                    return None;
                }
            };

            // Proceed to extract the package name and subdirectory
            let package_and_subdir = Path::new(&path)
                .parent()
                .unwrap_or(Path::new("none"))
                .iter()
                .skip_while(|&component| component.to_str() != Some("packages"))
                .skip(1) // Skip "packages" itself
                .fold(String::new(), |acc, curr| {
                    if acc.is_empty() {
                        curr.to_str().unwrap_or("").to_string()
                    } else {
                        format!("{}/{}", acc, curr.to_str().unwrap_or(""))
                    }
                });

            let package = package_and_subdir
                .split('/')
                .next()
                .unwrap_or_default()
                .to_string();

            let package_path = Path::new(&path)
                .iter()
                .take_while(|&component| component.to_str() != Some("sounds"))
                .fold(PathBuf::new(), |mut acc, curr| {
                    acc.push(curr);
                    acc
                });

            let sub_dir = Path::new(&package_and_subdir)
                .to_str()
                .unwrap_or("none")
                .split('/')
                .skip(1)
                .collect::<Vec<&str>>()
                .join("/");

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

            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");

            let num_channels = wave_data.num_channels;
            let outfile = format!("{output_bitrate}kb.{num_channels}ch.{hash}.webm");

            let mut output_path = PathBuf::from(&parsed.outdir);
            output_path.push(outfile.clone());

            let name = filename.to_string().replace(".wav", "");

            // update the bitrate if theres a bitrates file with the sound name in it
            let bitrates_path = package_path.join(".bitrates");
            let mut output_bitrate = output_bitrate;
            // Check if the bitrates file exists and is indeed a file
            if bitrates_path.is_file() {
                // Open the bitrates file
                let bitrates_file = match File::open(&bitrates_path) {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("Error opening bitrates file: {e}");
                        *success.lock().expect("Mutex poisoned") = false; // Set success to false if there's an error
                        return None;
                    }
                };
                let bitrates = std::io::BufReader::new(bitrates_file);

                // Iterate over each line, trimming whitespace and skipping empty lines
                for line in bitrates
                    .lines()
                    .filter_map(|line| line.ok())
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
                        output_bitrate = bitrate_str
                            .unwrap_or("96")
                            .parse()
                            .unwrap_or(output_bitrate);
                    }
                }
            }

            Some(SoundFileInfo {
                path: path.to_string_lossy().into_owned(),
                hash,
                name,
                outfile,
                package,
                lang,
                output_path: output_path.to_string_lossy().into_owned(),
                subdir: sub_dir,
                bitrate: output_bitrate,
                sample_rate: wave_data.format.sample_rate,
                num_samples: wave_data.num_samples,
                num_channels: wave_data.num_channels,
                duration: wave_data.duration,
                bits_per_sample: wave_data.format.bits_per_sample,
                audio_format: match wave_data.format.audio_format {
                    1 => "PCM".to_string(),
                    _ => "Unknown".to_string(),
                },
                byte_rate: wave_data.format.byte_rate,
                block_align: wave_data.format.block_align,
            })
        })
        .collect();

    let needs_conversion = sound_files_info
        .iter()
        .any(|info| info.sample_rate != 48000);
    for info in &sound_files_info {
        if info.sample_rate != 48000 {
            eprintln!(
                "Source file is not 48kHz: {} at {} hz",
                info.path, info.sample_rate
            );
            // ask user if they want to convert the file
            let proceed = parsed.yes
                || loop {
                    print!("Convert to 48kHz? (y/n): ");
                    io::stdout().flush().unwrap();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).unwrap();
                    match input.trim() {
                        "y" => break true,
                        "n" => break false,
                        _ => continue,
                    }
                };
            if !proceed {
                println!("Skipping file: {}", info.path);
                continue;
            }
            println!("Converting file: {}", info.path);

            let converted = info.path.replace("wav", "48000.wav");
            let output = Command::new("ffmpeg")
                .arg("-i")
                .arg(info.path.clone())
                .arg("-ar")
                .arg("48000")
                .arg(converted.clone())
                .arg("-y")
                .output()?;

            if !output.status.success() {
                eprintln!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "ffmpeg execution failed",
                ));
            }
            fs::remove_file(info.path.clone()).unwrap();
            fs::rename(converted, info.path.clone()).unwrap();
        }
    }
    if needs_conversion {
        eprintln!("Some files were converted to 48kHz. Please run the script again to encode the correct files.");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some source files needed to be converted to 48kHz",
        ));
    }

    let output_path = Path::new(".cache/info.json");
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
        // write_sound_info_to_json_by_package(output_path, &sound_files_info)?;
        write_sound_info_to_json(output_path, &sound_files_info)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to write sound info to JSON",
        ));
    }

    let results: Vec<Result<(), io::Error>> = sound_files_info
        .par_iter()
        .map(|info| run_ffmpeg(&parsed.ffmpeg, info, parsed.include_mp4))
        .collect();

    for (i, result) in results.iter().enumerate() {
        if let Err(e) = result {
            let infile = sound_files_info[i].path.clone();
            let outfile = sound_files_info[i].output_path.clone();
            eprintln!(
                "Error encoding sound file: {:?} to {:?} {e}",
                infile, outfile
            );
        }
    }

    // used to see the final network transfer size
    // let outfile = ".cache/info.json.gz";
    // compress_json_file(".cache/info.json", outfile)?;
    // let info = fs::metadata(outfile)?;
    // let human_readable = format_size(info.len(), DECIMAL);
    // println!("Compressed JSON file size: {human_readable}");

    let success = results.iter().all(Result::is_ok) && *success.lock().unwrap();

    // print_langs(sound_files_info.as_slice());
    let num_files = if parsed.include_mp4 {
        sound_files_info.len() * 2
    } else {
        sound_files_info.len()
    };

    let elapsed_time = start_time.elapsed();
    let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
    let elapsed = format_duration(elapsed_ms as u128);

    if success {
        println!("[encoder] encoded {num_files} sounds in {elapsed}");
    } else {
        eprintln!("[encoder] failure");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }

    Ok(())
}

fn format_duration(milliseconds: u128) -> String {
    if milliseconds < 1000 {
        return format!("{}ms", milliseconds);
    }

    let minutes = milliseconds / 60000;
    let seconds = (milliseconds % 60000) / 1000;
    let remaining_ms = milliseconds % 1000;

    match minutes {
        0 => format!("{}s {}ms", seconds, remaining_ms),
        _ => format!("{}m {}s {}ms", minutes, seconds, remaining_ms),
    }
}


fn run_ffmpeg(ffmpeg: &str, info: &SoundFileInfo, include_mp4: bool) -> io::Result<()> {
    // canonicalize the input and output paths
    let in_path = Path::new(&info.path).canonicalize()?;
    let out_path = Path::new(&info.output_path);
    if out_path.exists() {
        return Ok(());
    }
    println!("[encoder] processing {}", info.path);

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
        eprintln!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
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
    let output = Command::new("ffmpeg")
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
        eprintln!("ffmpeg error: {}", String::from_utf8_lossy(&output.stderr));
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
    println!("[encoder] languages: {:?}", langs);
}

fn compress_json_file(infile: &str, outfile: &str) -> Result<(), Error> {
    // Step 1: Read the JSON file
    let mut file = File::open(infile).expect("File not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read file");

    // Step 2: Deserialize JSON and then immediately serialize it to minify
    let v: Value = serde_json::from_str(&contents)?;
    let minified_json = serde_json::to_string(&v)?;

    // Step 3: Gzip the minified JSON
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(minified_json.as_bytes())
        .expect("Failed to write gzipped data");
    let compressed_data = encoder.finish().expect("Failed to compress");

    // Step 4: Save the gzipped data to a file
    let mut output = File::create(outfile).expect("Failed to create output file");
    output
        .write_all(&compressed_data)
        .expect("Failed to write to output file");
    Ok(())
}
