#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[macro_use]
mod logging;

use std::{
    collections::hash_map::DefaultHasher,
    env,
    fs::{self, DirEntry},
    hash::{Hash, Hasher},
    io::{self},
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use config::Config;
use info::Item;
use rayon::prelude::*;

mod wave;

use crate::logging::duration;

mod config;
mod info;
mod parser;

// Function to get the modification date as a String
fn get_modification_date_string<TPath: AsRef<Path>>(path: TPath) -> std::io::Result<String> {
    let metadata = fs::metadata(path)?;
    let modified_time = metadata.modified()?;
    // Convert SystemTime to a formatted string or a simple representation
    // Here, we convert it to UNIX timestamp for simplicity
    let datetime: DateTime<Utc> = modified_time.into();
    Ok(datetime.to_rfc3339())
}

fn main() -> io::Result<()> {
    let _display = logging::TimingsDisplay;
    let now = Instant::now();

    let parsed = time!("Parsing Args", {
        let args = env::args().collect::<Vec<String>>();
        parser::parse_args(&args)
    });
    logging::set_loglevel(parsed.loglevel);

    let config = time!("Loading Config", {
        let mut args = config::Args::parse();
        if args.config.is_none() {
            args.config = Some("scodefig.json".to_string());
        }
        let indir = args.indir.clone().unwrap_or(String::default());
        let config = args.config.clone().unwrap_or("scodefig.json".to_string());
        let config = Path::new(&indir).join(config);
        let config = config.to_str().unwrap_or("scodefig.json");
        debug!("Loading config from {config}");
        let config = config::Config::load(config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
            .unwrap_or_default()
            .merge_with_args(args);
        if config.indir.is_empty() {
            error!("No input directory specified");
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "No input directory specified",
            ));
        }
        config
    });

    debug!("{config}");
    debug!("{parsed:?}");

    if parsed.packages.is_empty() {
        info!("Encoding all packages");
    } else {
        info!("Encoding packages: {:?}", parsed.packages);
    };
    let items = time!("Create Items", { process(&config) })?;
    let encode_result = time!("Encode", { encode(config.clone(), &items) });
    if let Err(e) = encode_result {
        error!("{e}");
        return Err(e);
    }

    time!("Save cache", {
        let cache = info::Map::from_vec(items.clone());
        cache.save_cache_bin()?;
        if logging::is_debug() {
            cache.save_cache_json()?;
        }
    });
    let atlas = time!("Create Atlas", { info::AtlasMap::from_vec(&items) });
    time!("Save Atlas", {
        // atlas.save_json_v1(".cache")?;
        atlas.save_json_v2("encoded")?;
    });

    // debug!("{map:?}");
    success!("Done in {}", duration(now.elapsed().as_millis()));
    Ok(())
}

fn encode(config: Config, items: &[Item]) -> io::Result<()> {
    let items_to_encode: Vec<&info::Item> = time!("Check which sounds needs encoding", {
        items
            .par_iter()
            .filter(|info| !Path::new(&info.output_path).exists())
            .collect()
    });

    let results = time!("Encode sounds", {
        info!(
            "Encoding {} sounds out of {}",
            items_to_encode.len(),
            items.len()
        );
        encode_sounds(
            &items_to_encode,
            &config.ffmpeg.unwrap_or("ffmpeg".to_string()),
            config.include_mp4.unwrap_or(false),
        )
    });
    let mut failure = false;
    for (i, result) in results.iter().enumerate() {
        if let Err(e) = result {
            failure = true;
            let infile = items[i].path.clone();
            let outfile = items[i].output_path.clone();
            error!("Error encoding sound file: {infile:?} to {outfile:?} {e}");
        }
    }
    if failure {
        error!("Encoding Failure");
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn process(config: &Config) -> io::Result<Vec<Item>> {
    let package_names: Vec<String> = config.packages.keys().cloned().collect();
    let indir_path = Path::new(&config.indir);
    let join_with_indir = |package: &String| indir_path.join(package);
    let cache = info::Map::from_cache_bin().unwrap_or_default();
    let package_results: Vec<Result<Vec<Result<Item, io::Error>>, io::Error>> = package_names
        .par_iter()
        .map(|package_name| {
            let package_config = match config.packages.get(package_name) {
                Some(config) => config,
                None => {
                    let error_message = format!("Package {package_name} not found in config");
                    return Err(io::Error::new(io::ErrorKind::NotFound, error_message));
                }
            };
            let package_path = join_with_indir(package_name); // Assuming join_with_indir is defined elsewhere
            let package_sourcedir = package_config.sourcedir.clone().unwrap_or("sounds".to_string());
            let package_sourcedir_path = package_path.join(Path::new(&package_sourcedir));
            if !package_sourcedir_path.is_dir() {
                let error_message =
                    format!("Sourcedir: {package_sourcedir_path:?} is not a directory!",);
                return Err(io::Error::new(io::ErrorKind::NotFound, error_message));
            }
            let files = match fs::read_dir(package_sourcedir_path) {
                Ok(files) => files
                    .filter_map(std::result::Result::ok)
                    .collect::<Vec<DirEntry>>(),
                Err(e) => return Err(e),
            };
            let package_sources = package_config.sources.clone().unwrap_or_default();
            let items: Vec<Result<Item, io::Error>> = files
                .par_iter()
                .filter_map(|file| {
                    let file_buf = file.path();
                    if !file_buf.is_file() {
                        return None; // Skip directories or non-files
                    }
                    let file_path = file_buf.as_path();
                    let file_path_str = file_path.to_string_lossy();
                    let extension = file_path.extension().unwrap_or_default().to_string_lossy();
                    if extension != "wav" {
                        debug!("{file_path_str} is not wav");
                        return None; // Skip non-wav files
                    }

                    // Attempt to get the modification date, return Err wrapped in Some if fails
                    let modification_date = match get_modification_date_string(file_path) {
                        Ok(date) => date,
                        Err(e) => {
                            return Some(Err(e));
                        }
                    };

                    // should check the --skip-cache flag
                    if true {
                        let cached = cache.get(&file_path_str);
                        if let Some(cached) = cached {
                            debug!("Cached: {file_path_str}");
                            if modification_date == cached.modification_date {
                                return Some(Ok(cached.clone()));
                            }
                        }
                    }

                    // Existing logic for processing files
                    let file_name = file.file_name().to_string_lossy().replace(".wav", "");
                    let name = file_name;

                    // Wrap fs::read and wave processing in a Result::map_err to convert any error to io::Error
                    let result = fs::read(file_path)
                        .map_err(std::convert::Into::into)
                        .and_then(|buffer| {
                            wave::Data::from_buffer(&buffer)
                                .map_err(|e| {
                                    let original_msg = e.to_string();
                                    let msg =  format!("{original_msg} for file: {file_path_str}");
                                    io::Error::new(
                                        e.kind(),
                                        msg,
                                    )
                                })
                                .and_then(|wave| {
                                    // Use and_then to allow returning Err directly
                                    let sample_rate = wave.format.sample_rate;
                                    if sample_rate == 48000 {
                                        let input_samples = wave.num_samples;
                                        let input_channels = wave.format.num_channels;

                                        let mut hasher = DefaultHasher::new();
                                        buffer.hash(&mut hasher);
                                        let hash = hasher.finish().to_string();

                                        let (target_bitrate, target_channels) =
                                            package_sources.get(&name).map_or_else(
                                                || {
                                                    (
                                                        package_config
                                                            .bitrate
                                                            .unwrap_or(config.bitrate),
                                                        input_channels,
                                                    )
                                                },
                                                |settings| {
                                                    (
                                                        settings.bitrate.unwrap_or(
                                                            package_config
                                                                .bitrate
                                                                .unwrap_or(config.bitrate),
                                                        ),
                                                        settings.channels.unwrap_or(input_channels),
                                                    )
                                                },
                                            );

                                        let outfile = format!(
                                            "{target_bitrate}kb.{target_channels}ch.{hash}.webm"
                                        );
                                        let output_path = Path::new(&config.outdir).join(&outfile);

                                        Ok(Item {
                                            // Ensure to wrap the Item in Ok
                                            path: file_path_str.to_string(),
                                            name,
                                            outfile,
                                            package: package_name.to_string(),
                                            lang: "_".to_string(),
                                            sample_rate,
                                            num_samples: input_samples,
                                            num_channels: target_channels,
                                            modification_date,
                                            bitrate: target_bitrate,
                                            output_path: output_path.to_string_lossy().into_owned(),
                                        })
                                    } else {
                                        let message = format!("Sample rate {sample_rate} is not 48000 for file: {file_path_str}");
                                        Err(io::Error::new(
                                            io::ErrorKind::InvalidInput,
                                            message,
                                        ))
                                    }
                                })
                        });
                    Some(result)
                })
                .collect(); // Collect into Vec<Result<Item, io::Error>>
            Ok(items)
        })
        .collect();

    let mut ok_packages: Vec<Result<Item, io::Error>> = Vec::new();
    let mut err_packages: Vec<io::Error> = Vec::new();
    for package_result in package_results {
        match package_result {
            Ok(items) => ok_packages.extend(items),
            Err(e) => err_packages.push(e),
        }
    }

    if !err_packages.is_empty() {
        for e in err_packages {
            error!("{e}");
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Error in config packages",
        ));
    }

    let mut ok_values: Vec<Item> = Vec::new();
    let mut err_values: Vec<std::io::Error> = Vec::new();
    for item_result in ok_packages {
        match item_result {
            Ok(item) => ok_values.push(item),
            // Err(e) => err_values.push(e),
            Err(e) => err_values.push(e),
        };
    }
    if !err_values.is_empty() {
        let fixable: Vec<String> = err_values
            .iter()
            .filter_map(|e| {
                let kind = e.kind();
                let is_fixable =
                    kind == io::ErrorKind::InvalidInput || kind == io::ErrorKind::InvalidData;
                if is_fixable {
                    let string = e.to_string();
                    let path = string
                        .split("for file: ")
                        .nth(1)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    info!("{e}");
                    if path == String::default() {
                        warn!("Error message does not contain file path");
                        return None;
                    }
                    return Some(path);
                }
                None
            })
            .collect();
        if !fixable.is_empty() {
            warn!("The following files are not using pcm format and or sample rate 48000:");
            for file in &fixable {
                warn!("  {}", file);
            }
            if !config.yes.unwrap_or(false) {
                ask_to_reencode_source_files()?;
            }
            let ffmpeg = config.ffmpeg.clone().unwrap_or("ffmpeg".to_string());
            reencode_source_files(&fixable, &ffmpeg)?;
            info!(
                "Some files have be reencoded, rerunning the program to recheck the source files"
            );
            return process(config);
        }

        for e in err_values {
            error!("{e}");
        }
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Error in source items",
        ));
    }
    Ok(ok_values)
}

fn reencode_source_files(files: &[String], ffmpeg: &str) -> io::Result<()> {
    // Use a combination of `map` and `collect` to handle errors
    let results: Vec<io::Result<()>> = time!("Convert sample rates", {
        files
            .par_iter()
            .map(|file| {
                let converted = file.replace(".wav", ".48000.wav");
                debug!("Converting file: {file} to {converted}");
                let output = Command::new(ffmpeg)
                    .arg("-i")
                    .arg(file)
                    .arg("-ar")
                    .arg("48000")
                    .arg(&converted)
                    .arg("-acodec")
                    .arg("pcm_s16le")
                    .arg("-y")
                    .output()?;

                // Handle command execution error
                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    return Err(io::Error::new(io::ErrorKind::Other, error.to_string()));
                }
                fs::remove_file(file)?;
                fs::rename(&converted, file)?;
                Ok(())
            })
            .collect()
    });
    let errors = results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .collect::<Vec<&io::Error>>();
    if !errors.is_empty() {
        for error in errors {
            error!("{error}");
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Some files failed to encode",
        ));
    }
    Ok(())
}

// #[allow(clippy::too_many_lines)]
// fn run(parsed: &parser::ParsedArgs) -> io::Result<()> {
//     let output_bitrate = parsed.bitrate;
//     let wav_files: Vec<PathBuf> = time!("Find wav files", { find_wav_files_v1(&parsed.indir) });

//     let cached: info::Map = time!("Load cache", { load_cache(parsed.skip_cache) });

//     let paths: Vec<FilePath> = time!("Filter file paths", {
//         wav_files
//             .par_iter()
//             .filter_map(|path_buf| {
//                 let path = path_buf.as_path();
//                 let lossy = path.to_string_lossy().to_string();
//                 let package = lossy
//                     .split("packages")
//                     .nth(1)
//                     .unwrap_or("_")
//                     .split('/')
//                     .nth(1)
//                     .unwrap_or("_")
//                     .to_string();

//                 let package_path =
//                     lossy.split("packages").next().unwrap().to_string() + "packages/" + &package;
//                 let package_path = Path::new(package_path.as_str());

//                 if !parsed.packages.is_empty() && !parsed.packages.contains(&package) {
//                     return None;
//                 }

//                 Some(FilePath {
//                     path_buf: path_buf.clone(),
//                     package,
//                     package_path: package_path.to_path_buf(),
//                     lossy,
//                 })
//             })
//             .collect()
//     });

//     let finfo: Vec<Result<info::Item, io::Error>> = time!("Create sound info", {
//         paths
//             .clone()
//             .into_iter()
//             .map(|file_path| {
//                 let path_buf = file_path.path_buf.clone();
//                 let path = file_path.path_buf.as_path();
//                 let lossy = file_path.lossy.clone();
//                 let package = file_path.package.clone();
//                 let package_path = file_path.package_path.clone();

//                 // if the file has not been modified since the last time we hashed it
//                 // we use the cached info
//                 // downside is that if .lang or .bitrates files have been added or removed or changed
//                 // we won't know about it
//                 let modification_date = get_modification_date_string(path).unwrap_or_default();
//                 if let Some(cached_info) = cached.get(&lossy) {
//                     if modification_date == cached_info.modification_date {
//                         return Ok(cached_info.clone());
//                     }
//                 }

//                 let mut file = match File::open(path_buf) {
//                     Ok(file) => file,
//                     Err(e) => {
//                         return Err(io::Error::new(
//                             io::ErrorKind::Other,
//                             format!("Error opening file: {lossy} {e:?}"),
//                         ));
//                     }
//                 };
//                 let mut buffer = Vec::new();

//                 // this bit takes the longest time to run
//                 // we're using it to hash the entire file
//                 match file.read_to_end(&mut buffer).ok() {
//                     Some(_) => (),
//                     None => {
//                         return Err(io::Error::new(
//                             io::ErrorKind::Other,
//                             format!("Error reading file: {lossy}"),
//                         ));
//                     }
//                 }

//                 let mut hasher = DefaultHasher::new();
//                 buffer.hash(&mut hasher);
//                 let hash = hasher.finish().to_string();

//                 let wave_data = match Data::from_buffer(&buffer) {
//                     Ok(wave_data) => wave_data,
//                     Err(e) => {
//                         return Err(io::Error::new(
//                             io::ErrorKind::InvalidInput,
//                             format!("Error reading WAV data from {lossy}: {e:?}"),
//                         ));
//                     }
//                 };

//                 // let lang_start_time = Instant::now();
//                 // find a language file in the parent directory
//                 // for example src/packages/localisationprototype/sounds/en/.lang
//                 // that contains the language of the sound file
//                 // eg "english" or "french"
//                 let parent = path.parent().unwrap_or(Path::new(""));
//                 let lang_path = parent.join(".lang");
//                 let lang = if lang_path.is_file() {
//                     let mut lang_file = File::open(lang_path).unwrap();
//                     let mut lang = String::new();
//                     lang_file.read_to_string(&mut lang).unwrap();
//                     lang.trim().to_string()
//                 } else {
//                     "_".to_string()
//                 };
//                 // // lang_time += lang_start_time.elapsed().as_micros() as f32;

//                 // let strops_start_time = Instant::now();
//                 let filename = path.file_name().unwrap_or_default().to_str().unwrap_or("");

//                 let num_channels = wave_data.format.num_channels;
//                 let outfile = format!("{output_bitrate}kb.{num_channels}ch.{hash}.webm");

//                 let mut output_path = PathBuf::from(&parsed.outdir);
//                 output_path.push(outfile.clone());

//                 let name = filename.to_string().replace(".wav", "");
//                 // // strops_time += strops_start_time.elapsed().as_micros() as f32;

//                 // let bitrates_start_time = Instant::now();
//                 // update the bitrate if theres a bitrates file with the sound name in it
//                 let bitrates_path = package_path.join(".bitrates");
//                 let mut bitrate = output_bitrate;

//                 // Check if the bitrates file exists and is indeed a file
//                 if bitrates_path.is_file() {
//                     // Attempt to open the bitrates file, directly returning an Err variant of Result if it fails
//                     let bitrates_file = File::open(&bitrates_path);
//                     let bitrates = std::io::BufReader::new(bitrates_file.unwrap());

//                     // Iterate over each line, trimming whitespace and skipping empty lines
//                     for line in bitrates
//                         .lines()
//                         .map_while(Result::ok)
//                         .map(|line| line.trim_end().to_string())
//                         .filter(|line| !line.is_empty())
//                     {
//                         let mut parts = line.split_whitespace();
//                         let sound_name = parts.next().ok_or_else(|| {
//                             io::Error::new(
//                                 io::ErrorKind::Other,
//                                 "Missing sound name in bitrates file",
//                             )
//                         });
//                         let bitrate_str = parts.next().ok_or_else(|| {
//                             io::Error::new(io::ErrorKind::Other, "Missing bitrate in bitrates file")
//                         });
//                         if sound_name.is_err() || bitrate_str.is_err() {
//                             continue;
//                         }
//                         // Process the sound_name and bitrate_str...
//                         if sound_name.unwrap_or("_") == name {
//                             bitrate = bitrate_str.unwrap_or("96").parse().unwrap_or(bitrate);
//                         }
//                     }
//                 }
//                 // // bitrate_time += bitrates_start_time.elapsed().as_micros() as f32;

//                 Ok(info::Item {
//                     path: lossy.to_string(),
//                     name,
//                     outfile,
//                     package,
//                     lang,
//                     output_path: output_path.to_string_lossy().into_owned(),
//                     bitrate,
//                     sample_rate: wave_data.format.sample_rate,
//                     num_samples: wave_data.num_samples,
//                     modification_date,
//                     num_channels,
//                 })
//             })
//             .collect()
//     });

//     let had_error = time!("Check if had error in sound info", {
//         finfo.iter().any(Result::is_err)
//     });

//     time!("Check for errors in sound info", {
//         if had_error {
//             let files_that_can_be_fixed: Vec<String> = finfo
//                 .par_iter()
//                 .enumerate()
//                 .filter_map(|(i, result)| {
//                     if let Err(e) = result {
//                         let infile = paths[i].lossy.clone();
//                         if e.kind() == io::ErrorKind::InvalidInput {
//                             return Some(infile);
//                         }
//                     }
//                     None
//                 })
//                 .collect();
//             warn!("The following files are not using pcm format:");
//             for file in &files_that_can_be_fixed {
//                 warn!("  {}", file);
//             }
//             if !parsed.yes {
//                 ask_to_reencode_source_files()?;
//             }
//             let mut needs_rerun = false;
//             for (i, result) in finfo.iter().enumerate() {
//                 if let Err(e) = result {
//                     let infile = paths[i].lossy.clone();
//                     let outfile = infile.replace(".wav", ".pcm.wav");
//                     if e.kind() == io::ErrorKind::InvalidInput {
//                         info!("Converting file: {} to use pcm format", infile);
//                         let output = Command::new(parsed.ffmpeg.clone())
//                             .arg("-i")
//                             .arg(&infile)
//                             .arg("-ar")
//                             .arg("48000")
//                             .arg("-c:a") // Use "-c:a" to specify the audio codec
//                             .arg("pcm_s16le") // Set the codec to pcm_s16le
//                             .arg(&outfile)
//                             .arg("-y")
//                             .output()?;

//                         // Handle command execution error
//                         if !output.status.success() {
//                             let error = String::from_utf8_lossy(&output.stderr);
//                             return Err(io::Error::new(
//                                 io::ErrorKind::InvalidData,
//                                 error.to_string(),
//                             ));
//                         }
//                         fs::remove_file(&infile)?;
//                         fs::rename(&outfile, &infile)?;
//                         needs_rerun = true;
//                     }
//                 }
//             }
//             if needs_rerun {
//                 info!("Had to reencode some source files, rerunning the program to recheck the source files");
//                 return run(parsed);
//             }
//             let unfixable: Vec<String> = finfo
//                 .par_iter()
//                 .enumerate()
//                 .filter_map(|(i, result)| {
//                     if let Err(e) = result {
//                         let infile = paths[i].lossy.clone();
//                         if e.kind() != io::ErrorKind::InvalidInput {
//                             return Some(infile);
//                         }
//                     }
//                     None
//                 })
//                 .collect();
//             error!("The following files cannot be fixed:");
//             for file in &unfixable {
//                 error!("  {}", file);
//             }
//             for result in &finfo {
//                 if let Err(e) = result {
//                     error!("{e}");
//                 }
//             }
//             return Err(io::Error::new(
//                 io::ErrorKind::InvalidData,
//                 "Some files failed to encode",
//             ));
//         };
//     });

//     let items = finfo
//         .into_iter()
//         .filter_map(Result::ok)
//         .collect::<Vec<info::Item>>();
//     info!("Found {} source files", items.len());

//     let sounds_to_convert: Vec<&info::Item> = time!("Check sample rates", {
//         items
//             .iter()
//             .filter(|info| info.sample_rate != 48000)
//             .collect()
//     });

//     time!("Ask user for sample rates conversion", {
//         if !sounds_to_convert.is_empty() {
//             warn!("The following files have a sample rate other than 48 kHz:");
//             for info in &sounds_to_convert {
//                 warn!("  {}: {}", info.path, info.sample_rate);
//             }
//             if !parsed.yes {
//                 ask_to_reencode_source_files()?;
//             }
//         }
//     });

//     // Use a combination of `map` and `collect` to handle errors
//     let results: Vec<io::Result<()>> = time!("Convert sample rates", {
//         sounds_to_convert
//             .par_iter()
//             .map(|info| {
//                 info!("Converting file: {}", info.path);
//                 let converted = info.path.replace("wav", "48000.wav");
//                 let output = Command::new(parsed.ffmpeg.clone())
//                     .arg("-i")
//                     .arg(&info.path)
//                     .arg("-ar")
//                     .arg("48000")
//                     .arg(&converted)
//                     .arg("-y")
//                     .output()?;

//                 // Handle command execution error
//                 if !output.status.success() {
//                     let error = String::from_utf8_lossy(&output.stderr);
//                     return Err(io::Error::new(io::ErrorKind::Other, error.to_string()));
//                 }

//                 fs::remove_file(&info.path)?;
//                 fs::rename(&converted, &info.path)?;
//                 Ok(())
//             })
//             .collect()
//     });
//     if !results.is_empty() {
//         let mut had_error = false;
//         for (i, result) in results.iter().enumerate() {
//             if let Err(e) = result {
//                 let infile = sounds_to_convert[i].path.clone();
//                 error!("Error converting sound file: {infile:?} {e}");
//                 had_error = true;
//             }
//         }
//         if had_error {
//             error!("Sample rate conversion Failure");
//             return Err(io::Error::new(
//                 io::ErrorKind::InvalidData,
//                 "Some files failed to encode",
//             ));
//         }
//         info!("Since the sample rates were converted, the program will now rerun to recheck the source files");
//         return run(parsed);
//     }
//     time!("Save sound info to JSON", {
//         // info::AtlasMap::from_vec(&items).save_json_v1(".cache")?;
//         info::AtlasMap::from_vec(&items).save_json_v2(".cache")?;
//     });

//     if logging::is_debug() {
//         print_langs(&items);
//     }

//     let items_to_encode: Vec<&info::Item> = time!("Check which sounds needs encoding", {
//         if parsed.packages.is_empty() {
//             items
//                 .par_iter()
//                 .filter(|info| !Path::new(&info.output_path).exists())
//                 .collect()
//         } else {
//             items
//                 .par_iter()
//                 .filter(|info| {
//                     !Path::new(&info.output_path).exists()
//                         && parsed.packages.contains(&info.package)
//                 })
//                 .collect()
//         }
//     });

//     let results = time!("Encode sounds", {
//         encode_sounds(items_to_encode, &parsed.ffmpeg, parsed.include_mp4)
//     });
//     time!("Check encoding results", {
//         let mut failure = false;
//         for (i, result) in results.iter().enumerate() {
//             if let Err(e) = result {
//                 failure = true;
//                 let infile = items[i].path.clone();
//                 let outfile = items[i].output_path.clone();
//                 error!("Error encoding sound file: {infile:?} to {outfile:?} {e}");
//             }
//         }
//         if failure {
//             error!("Encoding Failure");
//             return Err(io::Error::new(
//                 io::ErrorKind::InvalidData,
//                 "Some files failed to encode",
//             ));
//         }
//     });
//     let map = time!("Create sound info map from vector of items", {
//         info::Map::from_vec(items)
//     });
//     time!("Save sound info to disk", {
//         map.save_cache_bin()?;
//     });
//     if logging::is_debug() {
//         time!("Save sound info to JSON", {
//             map.save_cache_json()?;
//         });
//     };
//     Ok(())
// }

fn ask_to_reencode_source_files() -> io::Result<()> {
    loop {
        success!("Do you want to reencode the source files? (y/n)");
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "y" {
            break;
        }
        if input.trim() == "n" {
            error!("Exiting");
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "User cancelled reencoding of source files",
            ));
        }
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

// fn print_langs(sound_info: &[info::Item]) {
//     let mut langs: Vec<String> = sound_info.iter().map(|info| info.lang.clone()).collect();
//     langs.sort();
//     langs.dedup();
//     debug!("Languages: {langs:?}");
// }

fn encode_sounds(
    sounds: &Vec<&info::Item>,
    ffmpeg: &str,
    include_mp4: bool,
) -> Vec<io::Result<()>> {
    let n = sounds.len();
    if n > 0 {
        let start = Instant::now();
        let ne = Arc::new(Mutex::new(0));
        let results: Vec<io::Result<()>> = sounds
            .par_iter()
            .map(|info| {
                *ne.lock().unwrap() += 1;
                logging::log_progress(start, *ne.lock().unwrap(), n);
                run_ffmpeg(ffmpeg, info, include_mp4)
            })
            .collect();
        logging::log_progress(start, n, n);
        results
    } else {
        vec![]
    }
}

// fn load_cache(skip: bool) -> info::Map {
//     if skip {
//         debug!("Skipping cache.");
//         info::Map::new()
//     } else {
//         let cache = info::Map::from_cache_bin();
//         if let Ok(cache) = cache {
//             debug!("Loaded {} cached items", cache.value.len());
//             cache
//         } else {
//             debug!("Failed to load cache, creating new cache");
//             info::Map::new()
//         }
//     }
// }
