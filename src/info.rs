use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::Path,
};

use crate::config::HashAlgorithm;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Item {
    pub path: String,
    pub name: String,
    pub outfile: String,
    pub package: String,
    pub lang: String,
    pub output_path: String,
    pub bitrate: u32,
    pub num_samples: usize,
    pub input_channels: u16,
    pub target_channels: u16,
    pub sample_rate: u32,
    pub modification_date: String,
    pub include_flac: bool,
    pub include_mp4: bool,
    pub hash: HashAlgorithm,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Map {
    // BTreeMap so the json cache is written in a stable order
    pub value: BTreeMap<String, Item>,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    pub fn new() -> Self {
        #![allow(unused_must_use)] // if it already exists, it's fine
        fs::create_dir_all(".cache");
        Map {
            value: BTreeMap::new(),
        }
    }

    // Method to insert a new SoundFileInfo into the map
    pub fn set(&mut self, key: String, info: Item) {
        self.value.insert(key, info);
    }

    pub fn get(&self, key: &str) -> Option<&Item> {
        self.value.get(key)
    }

    pub fn from_vec(vec: Vec<Item>) -> Self {
        vec.into_iter().fold(Map::new(), |mut map, info| {
            map.set(info.path.clone(), info);
            map
        })
    }

    pub fn from_map(map: BTreeMap<String, Item>) -> Self {
        Map { value: map }
    }

    pub fn from_cache_bin() -> io::Result<Self> {
        let mut file = File::open(".cache/info.bin")?;
        let mut encoded = Vec::new();
        file.read_to_end(&mut encoded)?;
        let value: BTreeMap<String, Item> = bincode::deserialize(&encoded)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Map::from_map(value))
    }

    pub fn save_cache_bin(&self) -> io::Result<&Self> {
        let encoded: Vec<u8> = bincode::serialize(&self.value)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let mut file = File::create(".cache/info.bin")?;
        file.write_all(&encoded)?;
        Ok(self)
    }
    pub fn save_cache_json(&self) -> io::Result<&Self> {
        let dir = Path::new(".cache");
        std::fs::create_dir_all(dir)?;
        let file = File::create(dir.join("info.json"))?;
        serde_json::to_writer_pretty(file, &self.value)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AtlasItem {
    name: String,
    file: String,
    nums: usize,  // num samples
    lang: String, // language
}

impl AtlasItem {
    pub fn from(info: &Item) -> Self {
        AtlasItem {
            name: info.name.clone(),
            file: info.outfile.clone(),
            nums: info.num_samples,
            lang: info.lang.clone(),
        }
    }
    fn format(&self) -> String {
        format!(
            "\n  [\"{}\", \"{}\", {}, \"{}\"]",
            self.name,
            self.file.replace(".webm", ""),
            self.nums,
            self.lang,
        )
    }
}

pub struct AtlasMap {
    // BTreeMap so packages are written in a stable order
    pub value: BTreeMap<String, Vec<AtlasItem>>,
}

impl AtlasMap {
    pub fn new() -> Self {
        AtlasMap {
            value: BTreeMap::new(),
        }
    }

    pub fn set(&mut self, key: String, info: AtlasItem) {
        self.value.entry(key).or_default().push(info);
    }

    pub fn from_vec(vec: &[Item]) -> Self {
        let mut map = vec.iter().fold(AtlasMap::new(), |mut map, info| {
            map.set(info.package.clone(), AtlasItem::from(info));
            map
        });
        // items come from directory listings whose order is not guaranteed,
        // sort them so the atlas is the same on every build
        for items in map.value.values_mut() {
            items.sort_by(|a, b| (&a.lang, &a.name, &a.file).cmp(&(&b.lang, &b.name, &b.file)));
        }
        map
    }

    pub fn save_json_v2(&self, dir: &str) -> io::Result<&Self> {
        let dirp = Path::new(dir);
        if !dirp.exists() {
            fs::create_dir_all(dirp)?;
        }
        let file = File::create(dirp.join(".atlas.json"))?;
        let mut writer = BufWriter::new(file);
        writeln!(writer, "{{")?;
        for (index, package) in self.value.iter().enumerate() {
            write!(writer, "\"{}\": [", package.0)?;
            for (index, item) in package.1.iter().enumerate() {
                write!(writer, "{}", item.format())?;
                // Comma between items, not after the last item
                if index < package.1.len() - 1 {
                    write!(writer, ", ")?;
                } else {
                    write!(writer, "\n]")?;
                }
            }
            // Handle commas between objects
            writeln!(
                writer,
                "{}",
                if index < self.value.len() - 1 {
                    ","
                } else {
                    ""
                }
            )?;
        }
        writeln!(writer, "}}")?;
        Ok(self)
    }
}
