use regex::Regex;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use walkdir::WalkDir;

fn main() -> io::Result<()> {
    let start_time = Instant::now(); // Record start time

    let args: Vec<String> = env::args().collect();
    let mut root_dir = String::new();
    let mut output_file = String::from("sound_files.txt"); // Default output file name

    for arg in args.iter().skip(1) {
        if arg.starts_with("--indir=") {
            root_dir = arg.trim_start_matches("--indir=").trim_matches('"').to_string();
        } else if arg.starts_with("--outfile=") {
            output_file = arg.trim_start_matches("--outfile=").trim_matches('"').to_string();
        }
    }

    if root_dir.is_empty() {
        println!("Usage: {} --indir=<directory> [--outfile=<output-file>]", args[0]);
        return Ok(());
    }

    // Compile a regular expression to match filenames ending with .wav
    let wav_regex = Regex::new(r"\.wav$").unwrap();

    // Initialize a vector to store the paths of sound files
    let mut sound_files: Vec<PathBuf> = Vec::new();

    // Walk the directory tree and store the paths of sound files in the vector
    for entry in WalkDir::new(&root_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(file_name) = entry.file_name().to_str() {
                if wav_regex.is_match(file_name) {
                    sound_files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    // Create directories leading up to the output file if they don't exist
    let output_path = Path::new(&output_file);
    if let Some(parent_dir) = output_path.parent() {
        fs::create_dir_all(parent_dir)?;
    }

    // Open a file for writing sound file names
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    // Write the names of sound files to the file
    for sound_file in &sound_files {
        writeln!(writer, "{}", sound_file.display())?;
    }

    // The file will be automatically closed when the writer goes out of scope
    let elapsed_time = start_time.elapsed(); // Calculate elapsed time
    let elapsed_ms = elapsed_time.as_secs() * 1000 + u64::from(elapsed_time.subsec_millis());
    println!("Time taken: {} ms", elapsed_ms); // Report time taken in milliseconds
    println!("Number of sound files found: {}", sound_files.len()); // Report number of sound files found

    Ok(())
}
