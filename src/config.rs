// args.rs

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::File, io::Read, path::Path};
use clap::Parser;

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
    #[clap(long, default_value = "scodefig.json")]
    pub config: String,
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
}
