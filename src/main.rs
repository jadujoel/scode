#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[macro_use]
mod logging;
use logging::Timer;

use std::{
    collections::hash_map::DefaultHasher,
    env,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::{self, BufRead, Read},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use rayon::prelude::*;
use walkdir::WalkDir;

mod wave;
use wave::Data;

mod config;
mod info;
mod parser;
mod timer;

#[derive(Debug, Clone)]
struct FilePath {
    path_buf: PathBuf,
    package: String,
    package_path: PathBuf,
    lossy: String,
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

fn main() -> io::Result<()> {
    let _timer = Timer::new("Main");
    let config = {
        let _timer = Timer::new("Loading Config");
        let args = config::Args::parse();
        let config = config::Config::load(&args.config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
            .merge_with_args(args);
        if config.indir.is_empty() {
            error!("No input directory specified");
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "No input directory specified",
            ));
        }
        config
    };
    let loglevel = {
        let _timer = Timer::new("Setting Loglevel");
        let logstring = config
            .loglevel
            .clone()
            .unwrap_or("info".to_string().clone());
        logging::LogLevel::from_str(&logstring).unwrap_or(logging::LogLevel::Info)
    };
    logging::set_loglevel(loglevel);

    let parsed = {
        let _timer = Timer::new("Parsing Args");
        let args = env::args().collect::<Vec<String>>();
        parser::parse_args(&args)
    };

    if parsed.packages.is_empty() {
        info!("Encoding all packages");
    } else {
        info!("Encoding packages: {:?}", parsed.packages);
    };
    run(&parsed, loglevel)?;
    success!("Done!");
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn run(parsed: &parser::ParsedArgs, loglevel: logging::LogLevel) -> io::Result<()> {
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
        let res = info::Map::from_cache_bin()?;
        debug!(
            "Loaded cache from disk in {} ms, {} microseconds per sound",
            now.elapsed().as_millis(),
            now.elapsed().as_micros() / num_sounds
        );
        res
    } else {
        debug!("No cache found, creating new cache");
        info::Map::new()
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
    let finfo: Vec<Result<info::Item, io::Error>> = paths
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
                "_".to_string()
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
                    if sound_name.unwrap_or("_") == name {
                        bitrate = bitrate_str.unwrap_or("96").parse().unwrap_or(bitrate);
                    }
                }
            }
            // // bitrate_time += bitrates_start_time.elapsed().as_micros() as f32;

            Ok(info::Item {
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
            return run(parsed, loglevel);
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
        .collect::<Vec<info::Item>>();

    info!("Found {} source files", finfo.len());
    debug!(
        "Filtered sound info in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    let sounds_to_convert: Vec<&info::Item> = finfo
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
        return run(parsed, loglevel);
    }

    debug!(
        "Checked sample rates in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    let now = Instant::now();
    info::AtlasMap::from_vec(&finfo).save_json_v1(".cache")?;
    info::AtlasMap::from_vec(&finfo).save_json_v2(".cache")?;
    debug!(
        "Wrote sound info to JSON in {} ms, {} microseconds per sound",
        now.elapsed().as_millis(),
        now.elapsed().as_micros() / num_sounds
    );

    if loglevel == logging::LogLevel::Debug {
        print_langs(&finfo);
    }

    let now = Instant::now();
    let num_sounds_encoded = Arc::new(Mutex::new(0));
    let sounds_that_needs_encoding: Vec<&info::Item> = if parsed.packages.is_empty() {
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
            logging::log_progress(ns, num_sounds_to_encode, now, loglevel);
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
    let map = info::Map::from_vec(finfo);
    map.save_cache_bin()?;
    debug!(
        "Saved sound info to disk in {} ms",
        now.elapsed().as_millis(),
    );

    if loglevel == logging::LogLevel::Debug {
        let now = Instant::now();
        map.save_cache_json()?;
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

fn run_ffmpeg(ffmpeg: &str, info: &info::Item, include_mp4: bool) -> io::Result<()> {
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

fn print_langs(sound_info: &[info::Item]) {
    let mut langs: Vec<String> = sound_info.iter().map(|info| info.lang.clone()).collect();
    langs.sort();
    langs.dedup();
    debug!("Languages: {langs:?}");
}
