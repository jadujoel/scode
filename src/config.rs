use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub indir: String,
    pub outdir: String,
    pub bitrate: u32,
    pub yes: Option<bool>,
    pub loglevel: Option<String>,
    pub packages: HashMap<String, Package>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
    pub sourcedir: Option<String>,
    pub bitrate: Option<u32>,
    #[serde(rename = "extends")]
    pub extends: Option<Vec<String>>,
    pub languages: Option<HashMap<String, String>>,
    pub sources: Option<HashMap<String, Source>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Source {
    pub bitrate: Option<u32>,
    pub channels: Option<u32>,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(long)]
    pub config: String,

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
            indir: args.indir.unwrap_or(self.indir),
            outdir: args.outdir.unwrap_or(self.outdir),
            bitrate: args.bitrate.unwrap_or(self.bitrate),
            yes: args.yes.or(self.yes),
            loglevel: args.loglevel.or(self.loglevel),
            packages: self.packages, // Assuming packages cannot be overridden by CLI
        }
    }
}
