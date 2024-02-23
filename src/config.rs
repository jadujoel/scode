use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub indir: String,
    pub outdir: String,
    pub bitrate: u32,
    pub yes: Option<bool>,
    pub loglevel: Option<String>,
    pub packages: HashMap<String, Package>,
    pub ffmpeg: Option<String>,
    pub include_mp4: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Package {
    pub sourcedir: Option<String>,
    pub bitrate: Option<u32>,
    pub extends: Option<Vec<String>>,
    pub languages: Option<HashMap<String, String>>,
    pub sources: Option<HashMap<String, Source>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Source {
    pub bitrate: Option<u32>,
    pub channels: Option<u16>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(long)]
    pub config: Option<String>,
    // Add optional command line arguments to override JSON configuration
    #[clap(long)]
    pub indir: Option<String>,
    #[clap(long)]
    pub outdir: Option<String>,
    #[clap(long)]
    pub bitrate: Option<u32>,
    #[clap(long)]
    pub yes: Option<bool>,
    #[clap(long)]
    pub loglevel: Option<String>,
    #[clap(long)]
    pub packages: Option<Vec<String>>,
    #[clap(long)]
    pub ffmpeg: Option<String>,
    #[clap(long)]
    pub include_mp4: Option<bool>,
}

impl Config {
    pub fn load(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let path = Path::new(config_path);
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: Config = serde_json::from_str(&contents)?;
        Ok(config)
    }
    pub fn merge_with_args(self, args: Args) -> Self {
        Config {
            indir: join_path(&args.indir.unwrap_or_default(), &self.indir),
            outdir: join_path(&args.outdir.unwrap_or_default(), &self.outdir),
            bitrate: args.bitrate.unwrap_or(self.bitrate),
            yes: args.yes.or(self.yes),
            loglevel: args.loglevel.or(self.loglevel),
            // filter packages by command line arguments
            packages: match args.packages {
                Some(packages) => self
                    .packages
                    .into_iter()
                    .filter(|(k, _)| packages.contains(k))
                    .collect(),
                None => self.packages,
            },
            ffmpeg: args.ffmpeg.or(self.ffmpeg),
            include_mp4: args.include_mp4.or(self.include_mp4).or(Some(false)),
        }
    }
}

impl std::default::Default for Config {
    fn default() -> Self {
        Config {
            indir: "packages".to_string(),
            outdir: "encoded".to_string(),
            bitrate: 96,
            yes: None,
            loglevel: None,
            packages: HashMap::new(),
            ffmpeg: Some("ffmpeg".to_string()),
            include_mp4: Some(false),
        }
    }
}

use std::fmt;

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Configuration Details")?;
        writeln!(f, "=====================")?;
        writeln!(f, "Input Directory: {}", self.indir)?;
        writeln!(f, "Output Directory: {}", self.outdir)?;
        writeln!(f, "Bitrate: {} kbps", self.bitrate)?;
        if let Some(yes) = self.yes {
            writeln!(
                f,
                "Automatic Yes to Prompts: {}",
                if yes { "Yes" } else { "No" }
            )?;
        }
        if let Some(ref loglevel) = self.loglevel {
            writeln!(f, "Log Level: {loglevel}")?;
        }
        writeln!(f, "Packages:")?;
        if self.packages.is_empty() {
            writeln!(f, "  [None]")?;
        } else {
            for (name, package) in &self.packages {
                writeln!(f, "  Name: {name}")?;
                if let Some(ref sourcedir) = package.sourcedir {
                    writeln!(f, "    Source Directory: {sourcedir}")?;
                }
                if let Some(bitrate) = package.bitrate {
                    writeln!(f, "    Bitrate: {bitrate} kbps")?;
                }
                if let Some(ref extends) = package.extends {
                    writeln!(f, "    Extends: {extends:?}")?;
                }
                if let Some(ref languages) = package.languages {
                    writeln!(f, "    Languages: {languages:?}")?;
                }
                if let Some(ref sources) = package.sources {
                    writeln!(f, "    Sources:")?;
                    for (src, source) in sources {
                        writeln!(f, "      {src}: {{")?;
                        if let Some(bitrate) = source.bitrate {
                            writeln!(f, "        Bitrate: {bitrate} kbps")?;
                        }
                        if let Some(channels) = source.channels {
                            writeln!(f, "        Channels: {channels}")?;
                        }
                        writeln!(f, "      }}")?;
                    }
                }
            }
        }
        writeln!(f, "=====================")?;
        Ok(())
    }
}

fn join_path(a: &str, b: &str) -> String {
    Path::new(a).join(b).to_str().unwrap_or("").to_string()
}
