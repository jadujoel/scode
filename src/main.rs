#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

#[macro_use]
mod logging;

mod test;

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    env,
    fs::{self, DirEntry},
    hash::{Hash, Hasher},
    io,
    path::Path,
    process::Command,
    sync::{Arc, Mutex},
    time::Instant,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use config::{Config, Source};
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

    let parsed = time!("Parse Args", {
        let args = env::args().collect::<Vec<String>>();
        parser::parse_args(&args)
    });
    logging::set_loglevel(parsed.loglevel);

    let config = time!("Load Config", {
        let mut args = config::Args::parse();
        if args.config.is_none() {
            args.config = Some("scodefig.jsonc".to_string());
        }
        let indir = args.indir.clone().unwrap_or(String::default());
        let config = args.config.clone().unwrap_or("scodefig.jsonc".to_string());
        let config = Path::new(&indir).join(config);
        let config = config.to_str().unwrap_or("scodefig.jsonc");
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

    info!("Input directory: {}", config.indir);
    info!("Output directory: {}", config.outdir);

    time!("Create output directory", {
        // check if it exists
        if !Path::new(&config.outdir).exists() {
            fs::create_dir_all(&config.outdir)?;
        }
    });

    if parsed.packages.is_empty() {
        info!("Encoding all packages");
    } else {
        info!("Encoding packages: {:?}", parsed.packages);
    };
    let items = time!("Create Items", { create_items(&config) })?;
    let encode_result = time!("Encode", { encode_items(config.clone(), &items) });
    if let Err(e) = encode_result {
        error!("{e}");
        return Err(e);
    }

    time!("Save Cache", {
        let cache = info::Map::from_vec(items.clone());
        cache.save_cache_bin()?;
        if logging::is_debug() {
            cache.save_cache_json()?;
        }
    });
    let atlas = time!("Create Atlas", { info::AtlasMap::from_vec(&items) });
    time!("Save Atlas", {
        // atlas.save_json_v1(".cache")?;
        atlas.save_json_v2(&config.outdir)?;
    });

    success!("Done in {}", duration(now.elapsed().as_millis()));
    Ok(())
}

static NO_LANG: &str = "_";

#[allow(clippy::too_many_lines)]
fn create_items(config: &Config) -> io::Result<Vec<Item>> {
    let package_names: Vec<String> = config.packages.keys().cloned().collect();
    let indir_path = Path::new(&config.indir);
    let join_with_indir = |package: &String| indir_path.join(package);
    let use_cache = config.use_cache.unwrap_or(true);
    let cache = if use_cache {
        debug!("Loading cache");
        info::Map::from_cache_bin().unwrap_or_default()
    } else {
        debug!("Skipping cache");
        info::Map::new()
    };
    let package_results: Vec<Result<Vec<Result<Item, io::Error>>, io::Error>> = package_names
        .par_iter()
        .map(|package_name| {
            let Some(package_config) = config.packages.get(package_name) else {
                let error_message = format!("Package {package_name} not found in config");
                return Err(io::Error::new(io::ErrorKind::NotFound, error_message));
            };
            let package_path = join_with_indir(package_name); // Assuming join_with_indir is defined elsewhere
            let package_sourcedir = package_config
                .sourcedir
                .clone()
                .unwrap_or("sounds".to_string());
            let package_sourcedir_path = package_path.join(Path::new(&package_sourcedir));
            if !package_sourcedir_path.is_dir() {
                let error_message =
                    format!("Sourcedir: {package_sourcedir_path:?} is not a directory!",);
                return Err(io::Error::new(io::ErrorKind::NotFound, error_message));
            }

            if package_config.languages.is_none() {
                let files = match fs::read_dir(package_sourcedir_path) {
                    Ok(files) => files
                        .filter_map(std::result::Result::ok)
                        .collect::<Vec<DirEntry>>(),
                    Err(e) => return Err(e),
                };
                let package_sources = package_config.sources.clone().unwrap_or_default();

                // look through the language folders

                let items: Vec<Result<Item, io::Error>> = files
                    .par_iter()
                    .filter_map(|file| {
                        create_item_for_file(
                            file,
                            &package_sources,
                            package_config,
                            package_name,
                            config,
                            use_cache,
                            &cache,
                            &NO_LANG.to_string(),
                        )
                    })
                    .collect(); // Collect into Vec<Result<Item, io::Error>>
                return Ok(items);
            }
            let langs = package_config.languages.clone().unwrap();
            let mut items: Vec<Result<Item, io::Error>> = Vec::new();
            for (lang, lang_dir) in langs {
                let lang_path = package_sourcedir_path.join(Path::new(&lang_dir));
                if !lang_path.is_dir() {
                    let error_message = format!("Language dir: {lang_path:?} is not a directory!",);
                    return Err(io::Error::new(io::ErrorKind::NotFound, error_message));
                }
                let files = match fs::read_dir(lang_path) {
                    Ok(files) => files
                        .filter_map(std::result::Result::ok)
                        .collect::<Vec<DirEntry>>(),
                    Err(e) => return Err(e),
                };
                let package_sources = package_config.sources.clone().unwrap_or_default();
                let lang_items: Vec<Result<Item, io::Error>> = files
                    .par_iter()
                    .filter_map(|file| {
                        create_item_for_file(
                            file,
                            &package_sources,
                            package_config,
                            package_name,
                            config,
                            use_cache,
                            &cache,
                            &lang,
                        )
                    })
                    .collect(); // Collect into Vec<Result<Item, io::Error>>
                items.extend(lang_items);
            }
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
            info!("Some files had be reencoded, rerunning the program to recheck the source files");
            return create_items(config);
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

fn create_item_for_file(
    file: &DirEntry,
    package_sources: &HashMap<String, Source>,
    package_config: &config::Package,
    package_name: &String,
    config: &config::Config,
    use_cache: bool,
    cache: &info::Map,
    lang: &String,
) -> Option<Result<Item, io::Error>> {
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
    if use_cache {
        let cached = cache.get(&file_path_str);
        if let Some(cached) = cached {
            debug!("Cached: {file_path_str}");
            if modification_date == cached.modification_date {
                return Some(Ok(cached.clone()));
            }
        }
    }

    let name = file.file_name().to_string_lossy().replace(".wav", "");

    // Wrap fs::read and wave processing in a Result::map_err to convert any error to io::Error
    let result = fs::read(file_path)
        .map_err(std::convert::Into::into)
        .and_then(|buffer| {
            wave::Data::from_buffer(&buffer)
                .map_err(|e| {
                    let original_msg = e.to_string();
                    let msg = format!("{original_msg} for file: {file_path_str}");
                    io::Error::new(e.kind(), msg)
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
                        // convert to be maximum15 characters
                        let hash = &hash[..15];
                        let (target_bitrate, target_channels) =
                            package_sources.get(&name).map_or_else(
                                || {
                                    (
                                        package_config.bitrate.unwrap_or(config.bitrate),
                                        input_channels,
                                    )
                                },
                                |settings| {
                                    (
                                        settings.bitrate.unwrap_or(
                                            package_config.bitrate.unwrap_or(config.bitrate),
                                        ),
                                        settings.channels.unwrap_or(input_channels),
                                    )
                                },
                            );

                        let outfile = format!("{target_bitrate}kb.{target_channels}ch.{hash}.webm");
                        let output_path = Path::new(&config.outdir).canonicalize()?.join(&outfile);

                        Ok(Item {
                            // Ensure to wrap the Item in Ok
                            path: file_path_str.to_string(),
                            name,
                            outfile,
                            package: package_name.to_string(),
                            lang: lang.to_string(),
                            sample_rate,
                            num_samples: input_samples,
                            input_channels,
                            target_channels,
                            modification_date,
                            bitrate: target_bitrate,
                            output_path: output_path.to_string_lossy().into_owned(),
                        })
                    } else {
                        let message = format!(
                            "Sample rate {sample_rate} is not 48000 for file: {file_path_str}"
                        );
                        Err(io::Error::new(io::ErrorKind::InvalidInput, message))
                    }
                })
        });
    Some(result)
}

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
                    .arg("pcm_s24le")
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

fn encode_items(config: Config, items: &[Item]) -> io::Result<()> {
    let items_to_encode: Vec<&info::Item> = time!("Encode: Check need", {
        items
            .par_iter()
            .filter(|info| !Path::new(&info.output_path).exists())
            .collect()
    });
    time!("Encode: Check ffmpeg exists", {
        let ffmpeg = config.ffmpeg.clone().unwrap_or("ffmpeg".to_string());
        // check if ffmpeg is installed
        let ffmpeg_check = Command::new(&ffmpeg).arg("-version").output();
        if let Err(e) = ffmpeg_check {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("ffmpeg not found at {}: {e}", &ffmpeg),
            ));
        }
    });
    let results = time!("Encode: Sounds", {
        info!(
            "Encoding {} sounds out of {}",
            items_to_encode.len(),
            items.len()
        );
        encode_with_progress(
            &items_to_encode,
            &config.ffmpeg.unwrap_or("ffmpeg".to_string()),
            config.include_mp4.unwrap_or(false),
            config.include_flac.unwrap_or(false),
            config.include_webm.unwrap_or(true),
            config.include_opus.unwrap_or(false),
        )
    });
    let errors = results
        .par_iter()
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

fn encode_with_progress(
    sounds: &Vec<&info::Item>,
    ffmpeg: &str,
    include_mp4: bool,
    include_flac: bool,
    include_webm: bool,
    include_opus: bool,
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
                encode_one_item(
                    ffmpeg,
                    info,
                    include_mp4,
                    include_flac,
                    include_webm,
                    include_opus,
                )
            })
            .collect();
        logging::log_progress(start, n, n);
        results
    } else {
        vec![]
    }
}

fn encode_one_item(
    ffmpeg: &str,
    info: &info::Item,
    include_mp4: bool,
    include_flac: bool,
    include_webm: bool,
    include_opus: bool,
) -> io::Result<()> {
    let infile = Path::new(&info.path);
    let infile = match infile.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {}", infile.to_string_lossy()),
            ));
        }
    }
    .to_string_lossy()
    .to_string();

    let out_path = Path::new(&info.output_path);
    let outfile = out_path.to_string_lossy().to_string();

    debug!("Encoding {infile}");
    debug!("Encoding {outfile}");

    let is_stereo_to_mono = info.input_channels == 2 && info.target_channels == 1;

    // When specifying the bitrate in FFmpeg for audio encoding,
    // you should specify the total bitrate for all channels, not per channel.
    // The bitrate you set with commands like -b:a for audio streams is applied to the entire audio stream,
    // encompassing all its channels. For example, if you specify a bitrate of 128k (128 kbps),
    // this bitrate is the total bitrate used for the audio stream,
    // whether it's mono, stereo, or multi-channel audio.
    let bitrate = info.bitrate * u32::from(info.target_channels);

    let mut command = Command::new(ffmpeg);
    let command = command
        .arg("-i")
        .arg(infile)
        .arg("-b:a")
        .arg(bitrate.to_string() + "k")
        .arg("-ar")
        .arg("48000")
        // remove any metadata
        .arg("-map_metadata")
        .arg("-1")
        .arg("-y");
    // opus codec
    let command = if is_stereo_to_mono {
        command
            // mono mixdown with gain adjustment
            .arg("-af")
            .arg("pan=mono|c0=0.5*c0+0.5*c1")
            .arg("-ac")
            .arg("1")
    } else {
        command
    };

    if include_webm {
        let result = command.arg("-c:a").arg("libopus").arg(&outfile).output();

        if let Err(e) = result {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("ffmpeg execution failed when encoding webm file {outfile} with error {e}",),
            ));
        }

        let output = result.unwrap();
        let status = output.status;
        if !status.success() {
            warn!("command: {command:?}");
            warn!("webm_output: {output:?}");
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding webm file {outfile} with status {status}",
                ),
            ));
        }
    }

    if include_opus {
        let outfile = outfile.clone().replace("webm", "opus");
        debug!("Encoding {outfile}");

        // write the flac file
        let result = command
            .arg("-c:a")
            .arg("libopus")
            .arg(outfile.clone())
            .output();
        if let Err(e) = result {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding flac file {} with error {e}",
                    outfile.clone()
                ),
            ));
        }
        let status = result.unwrap().status;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding flac file {outfile} with status {status}",
                ),
            ));
        }
    }

    if include_mp4 {
        let outfile = outfile.clone().replace("webm", "mp4");
        debug!("Encoding {outfile}");

        // write the mp4 file
        let result = command
            .arg("-c:a")
            .arg("aac")
            .arg("-movflags")
            .arg("+faststart")
            .arg(outfile.clone())
            .output();
        if let Err(e) = result {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding mp4 file {} with error {e}",
                    outfile.clone()
                ),
            ));
        }
        let status = result.unwrap().status;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding mp4 file {outfile} with status {status}",
                ),
            ));
        }
    }

    if include_flac {
        let outfile = outfile.clone().replace("webm", "flac");
        debug!("Encoding {outfile}");

        // write the flac file
        let result = command
            .arg("-c:a")
            .arg("flac")
            .arg(outfile.clone())
            .output();
        if let Err(e) = result {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding flac file {} with error {e}",
                    outfile.clone()
                ),
            ));
        }
        let status = result.unwrap().status;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "ffmpeg execution failed when encoding flac file {outfile} with status {status}",
                ),
            ));
        }
    }

    Ok(())
}
