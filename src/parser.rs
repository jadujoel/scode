use crate::logging::LogLevel;


#[allow(clippy::struct_excessive_bools)]
#[derive(Debug)]
pub struct ParsedArgs {
    pub indir: String,
    pub outdir: String,
    pub ffmpeg: String,
    pub packages: Vec<String>,
    pub include_mp4: bool,
    pub bitrate: u32,
    pub yes: bool,
    pub skip_cache: bool,
    pub loglevel: LogLevel,
    pub help: bool,
}
pub fn parse_args(args: &[String]) -> ParsedArgs {
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

    ParsedArgs {
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
