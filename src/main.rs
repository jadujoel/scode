use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

use rayon::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

mod wave;
use wave::WaveData;

// Example:
#[derive(Serialize, Deserialize, Debug)]
struct SoundFileInfo {
    path: String,
    name: String,
    hash: String,
    // is_cached: bool,
    // cached_path: String,
    package: String,
    output_path: String,
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
    println!("Writing sound info to {output_file}");

    // Write TypeScript file with sound file information
    let file = File::create(output_file)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "[")?;
    for (index, info) in sound_files_info.iter().enumerate() {
        // Write sound information
        write!(
            writer,
            "  {{ \"name\": \"{}\", \"path\": \"{}\", \"output_path\": \"{}\", \"package\": \"{}\", \"hash\": \"{}\", \"num_samples\": {}, \"duration\": {}, \"audio_format\": \"{}\", \"num_channels\": {}, \"sample_rate\": {}, \"byte_rate\": {}, \"block_align\": {}, \"bits_per_sample\": {}",
            info.name,
            info.path,
            info.output_path,
            info.package,
            info.hash,
            info.num_samples,
            info.duration,
            info.audio_format,
            info.num_channels,
            info.sample_rate,
            info.byte_rate,
            info.block_align,
            info.bits_per_sample,
        )?;

        // Check for duplicates
        let duplicates: Vec<&SoundFileInfo> = sound_files_info
            .iter()
            .filter(|&other| other.hash == info.hash && other.path != info.path)
            .collect();

        // If duplicates exist, add duplicates field
        if !duplicates.is_empty() {
            write!(writer, ", \"duplicates\": [")?;
            for (i, dup) in duplicates.iter().enumerate() {
                if i > 0 {
                    write!(writer, ", ")?;
                }
                write!(writer, "\"{}\"", dup.path)?;
            }
            write!(writer, "]")?;
        }
        // Close the sound entry
        writeln!(
            writer,
            " }}{}",
            if index < sound_files_info.len() - 1 {
                ","
            } else {
                ""
            }
        )?;
    }
    writeln!(writer, "]")?;

    Ok(())
}

struct Args {
    root_dir: String,
    output_file: String,
    ts_output_file: String,
}

fn parse_args(args: &[String]) -> Args {
    let mut root_dir = String::new();
    let mut output_file = String::from("out/sounds.txt"); // Default output file name
    let mut ts_output_file = String::from("out/sounds.ts"); // Default TypeScript output file name

    for arg in args.iter().skip(1) {
        if arg.starts_with("--indir=") {
            root_dir = arg
                .trim_start_matches("--indir=")
                .trim_matches('"')
                .to_string();
        } else if arg.starts_with("--outfile=") {
            output_file = arg
                .trim_start_matches("--outfile=")
                .trim_matches('"')
                .to_string();
        } else if arg.starts_with("--tsoutfile=") {
            ts_output_file = arg
                .trim_start_matches("--tsoutfile=")
                .trim_matches('"')
                .to_string();
        }
    }

    Args {
        root_dir,
        output_file,
        ts_output_file,
    }
}


#[allow(clippy::too_many_lines)]
fn main() -> io::Result<()> {
    let start_time = Instant::now(); // Record start time
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(args.as_ref());
    let root_dir = parsed.root_dir;
    if root_dir.is_empty() {
        println!(
            "Usage: {} --indir=<directory> [--outfile=<output-file>] [--tsoutfile=<typescript-output-file>]",
            args[0]
        );
        return Ok(());
    }

    let mut cached_file: File;
    let cached_path = Path::new(".cache/sounds.bin");
    let mut encoded = Vec::new();
    let mut cached: Vec<SoundFileInfo>;

    if let Some(parent_dir) = cached_path.parent() {
        fs::create_dir_all(parent_dir)?;
        cached = Vec::new();
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to create output directory",
        ));
    }
    if Path::new(cached_path).is_file() {
        println!("Cache exists!");
        cached_file = File::open(cached_path)?;
        cached_file.read_to_end(&mut encoded)?;
        cached = bincode::deserialize(&encoded).unwrap();
    } else {
        println!("No Cache found.");

    }
    println!("{:#?}", cached);

    // Compile a regular expression to match filenames ending with .wav
    let wav_regex = Regex::new(r"\.wav$").unwrap();

    // Initialize a Mutex-wrapped HashSet to store encountered hashes
    let hash_set: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // Walk the directory tree and calculate hash for each sound file
    let sound_files_info: Vec<SoundFileInfo> = WalkDir::new(&root_dir)
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
        .collect::<Vec<PathBuf>>()
        .par_iter()
        .filter_map(|path_buf| {
            // Normalize the path to resolve any relative components
            let path = fs::canonicalize(path_buf).ok()?;
            let mut file = File::open(path_buf).ok()?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).ok()?;
            let mut hasher = DefaultHasher::new();
            buffer.hash(&mut hasher);
            let hash = hasher.finish().to_string();
            // Check if hash already exists in the HashSet
            let mut locked_hash_set = hash_set.lock().unwrap();
            if !locked_hash_set.contains(&hash) {
                locked_hash_set.insert(hash.clone());
            }

            let wave_data = WaveData::from_buffer(&buffer).ok()?;

            // Proceed to extract the package name and subdirectory
            let package_and_subdir = path
                .iter()
                .skip_while(|&component| component.to_str() != Some("packages"))
                .skip(1) // Skip "packages" itself
                .take_while(|&component| component.to_str() != Some("sounds"))
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

            // let bitrates_file = path.with_file_name("bitrates");
            // println!("Bitrates file found: {:?} {:?}", bitrates_file.file_stem(), bitrates_file.file_name());
            // if bitrates_file.exists() {
            // }

            let sub_dir = package_and_subdir
                .split('/')
                .skip(1)
                .collect::<Vec<&str>>()
                .join("/");

            let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");
            let output_file_name = format!("{}_{}.webm", filename.trim_end_matches(".wav"), hash);
            let output_path = if sub_dir.is_empty() {
                format!("encoded/{package}/{output_file_name}")
            } else {
                format!("encoded/{package}/{sub_dir}/{output_file_name}")
            };

            Some(SoundFileInfo {
                path: path.to_string_lossy().into_owned(),
                hash,
                name: filename.to_string().replace(".wav", ""),
                package,
                output_path,
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

    let output_path = Path::new(".cache/info.json");
    if let Some(parent_dir) = output_path.parent() {
        fs::create_dir_all(parent_dir)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to create output directory",
        ));
    }

    if let Some(output_path) = output_path.to_str() {
        write_sound_info_to_json(output_path, &sound_files_info)?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Failed to write sound info to JSON",
        ));
    }

    let cached: Vec<u8> = bincode::serialize(&sound_files_info).unwrap();
    let mut file = File::create(".cache/sounds.bin")?;
    file.write_all(&cached)?;

    // The files will be automatically closed when the writers go out of scope
    let elapsed_time = start_time.elapsed(); // Calculate elapsed time
    let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
    println!("Time taken: {elapsed_ms} ms"); // Report time taken in milliseconds
    println!("Number of sound files found: {}", sound_files_info.len()); // Report number of sound files found

    Ok(())
}
