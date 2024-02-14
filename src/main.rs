use rayon::prelude::*;
use regex::Regex;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use walkdir::WalkDir;

#[derive(Debug)]
struct SoundFileInfo {
    path: String,
    hash: String,
}

fn write_sound_info_to_ts(
    ts_output_file: &str,
    sound_files_info: &[SoundFileInfo],
) -> io::Result<()> {
    // Write TypeScript file with sound file information
    let ts_file = File::create(ts_output_file)?;
    let mut ts_writer = BufWriter::new(ts_file);

    writeln!(ts_writer, "const sounds = [")?;
    for (index, info) in sound_files_info.iter().enumerate() {
        // Write sound information
        write!(
            ts_writer,
            "  {{ path: \"{}\", hash: \"{}\"",
            info.path, info.hash
        )?;

        // Check for duplicates
        let duplicates: Vec<&SoundFileInfo> = sound_files_info
            .iter()
            .filter(|&other| other.hash == info.hash && other.path != info.path)
            .collect();

        // If duplicates exist, add duplicates field
        if !duplicates.is_empty() {
            write!(ts_writer, ", duplicates: [")?;
            for (i, dup) in duplicates.iter().enumerate() {
                if i > 0 {
                    write!(ts_writer, ", ")?;
                }
                write!(ts_writer, "\"{}\"", dup.path)?;
            }
            write!(ts_writer, "]")?;
        }
        // Close the sound entry
        writeln!(
            ts_writer,
            " }}{}",
            if index < sound_files_info.len() - 1 {
                ","
            } else {
                ""
            }
        )?;
    }

    writeln!(ts_writer, "];")?;

    Ok(())
}

fn write_sound_info_to_file(output_path: &Path, sound_files_info: &[SoundFileInfo]) -> io::Result<()> {
    // Open a file for writing sound file names and hashes
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Write the names of sound files and their hashes to the file
    for info in sound_files_info {
        writeln!(writer, "path: {}, hash: {}", info.path, info.hash)?;
    }

    Ok(())
}


struct Args {
    root_dir: String,
    output_file: String,
    ts_output_file: String,
}

fn parse_args(args: Vec<String>) -> Args {
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

fn main() -> io::Result<()> {
    let start_time = Instant::now(); // Record start time
    let args: Vec<String> = env::args().collect();
    let parsed = parse_args(args.clone());
    let root_dir = parsed.root_dir;
    let output_file = parsed.output_file;
    let ts_output_file = parsed.ts_output_file;

    if root_dir.is_empty() {
        println!(
            "Usage: {} --indir=<directory> [--outfile=<output-file>] [--tsoutfile=<typescript-output-file>]",
            args[0]
        );
        return Ok(());
    }

    // Compile a regular expression to match filenames ending with .wav
    let wav_regex = Regex::new(r"\.wav$").unwrap();

    // Initialize a vector to store sound file information
    let mut sound_files_info: Vec<SoundFileInfo> = Vec::new();
    // Initialize a Mutex-wrapped HashSet to store encountered hashes
    let hash_set: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // Walk the directory tree and calculate hash for each sound file
    sound_files_info = WalkDir::new(&root_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if entry.file_type().is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if wav_regex.is_match(file_name) {
                        Some(entry.path().to_path_buf())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<PathBuf>>()
        .par_iter()
        .filter_map(|path| {
            let mut file = File::open(path).ok()?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer).ok()?;
            let mut hasher = DefaultHasher::new();
            buffer.hash(&mut hasher);
            let hash = hasher.finish();
            let hash_str = format!("{:x}", hash);

            // Check if hash already exists in the HashSet
            let mut locked_hash_set = hash_set.lock().unwrap();
            if !locked_hash_set.contains(&hash_str) {
                // Insert hash into HashSet
                locked_hash_set.insert(hash_str.clone());
            }

            Some(SoundFileInfo {
                path: path.to_string_lossy().into_owned(),
                hash: hash_str,
            })
        })
        .collect();

    // Create directories leading up to the output file if they don't exist
    let output_path = Path::new(&output_file);
    if let Some(parent_dir) = output_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    let ts_output_path = Path::new(&ts_output_file);
    if let Some(parent_dir) = ts_output_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    write_sound_info_to_file(output_path, &sound_files_info)?;
    write_sound_info_to_ts(&ts_output_file, &sound_files_info)?;

    // The files will be automatically closed when the writers go out of scope
    let elapsed_time = start_time.elapsed(); // Calculate elapsed time
    let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
    println!("Time taken: {} ms", elapsed_ms); // Report time taken in milliseconds
    println!("Number of sound files found: {}", sound_files_info.len()); // Report number of sound files found

    Ok(())
}
