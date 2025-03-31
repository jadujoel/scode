use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufWriter, Read, Write},
    path::Path,
};

use crate::wave;

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
    pub include_flac: bool
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewItem {
    pub path: String,
    pub name: String,
    pub outfile: String,
    pub package: String,
    pub lang: String,
    pub output_path: String,
    pub bitrate: u32,
    pub modification_date: String,
    pub wave_data: wave::Data,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Map {
    pub value: HashMap<String, Item>,
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
            value: HashMap::new(),
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

    pub fn from_map(map: HashMap<String, Item>) -> Self {
        Map { value: map }
    }

    pub fn from_cache_bin() -> io::Result<Self> {
        let mut file = File::open(".cache/info.bin")?;
        let mut encoded = Vec::new();
        file.read_to_end(&mut encoded)?;
        let value: HashMap<String, Item> = bincode::deserialize(&encoded)
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

    // pub fn from_cache_json() -> io::Result<Self> {
    //     let file = File::open(".cache/info.json")?;
    //     let value: HashMap<String, Item> = serde_json::from_reader(file)
    //         .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    //     Ok(Map::from_map(value))
    // }

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
    pub value: HashMap<String, Vec<AtlasItem>>,
}

impl AtlasMap {
    pub fn new() -> Self {
        AtlasMap {
            value: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, info: AtlasItem) {
        self.value.entry(key).or_default().push(info);
    }

    pub fn from_vec(vec: &[Item]) -> Self {
        vec.iter().fold(AtlasMap::new(), |mut map, info| {
            map.set(info.package.clone(), AtlasItem::from(info));
            map
        })
    }

    // pub fn save_json_v1(&self, dir: &str) -> io::Result<&Self> {
    //     let dirp = Path::new(dir);
    //     if !dirp.exists() {
    //         fs::create_dir_all(dirp)?;
    //     }
    //     let file = File::create(dirp.join(".atlas.json"))?;
    //     serde_json::to_writer_pretty(file, &self.value)
    //         .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    //     Ok(self)
    // }

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

// fn save_atlas(items: &[Item], output_file: &str) -> io::Result<()> {
//     let file = File::create(output_file)?;
//     let mut writer = BufWriter::new(file);
//     let mut packages: HashMap<String, Vec<&AtlasItem>> = HashMap::new();
//     for info in items {
//         packages
//             .entry(info.package.clone())
//             .or_default()
//             .push(info);
//     }

//     writeln!(writer, "{{")?;
//     for (index, package) in packages.iter().enumerate() {
//         write!(writer, "\"{}\": [", package.0)?;
//         for (index, info) in package.1.iter().enumerate() {
//             if info.lang == "none" {
//                 // If lang is "none", skip the lang field
//                 write!(
//                     writer,
//                     "\n  [\"{}\", \"{}\", {}]",
//                     info.name,
//                     info.outfile.replace(".webm", ""),
//                     info.num_samples,
//                 )?;
//             } else {
//                 // If lang is not "none", include it in the JSON
//                 write!(
//                     writer,
//                     "\n  [\"{}\", \"{}\", {}, \"{}\"]",
//                     info.name,
//                     info.outfile.replace(".webm", ""),
//                     info.num_samples,
//                     info.lang,
//                 )?;
//             }

//             // Comma between items, not after the last item
//             if index < package.1.len() - 1 {
//                 write!(writer, ", ")?;
//             } else {
//                 write!(writer, "\n]")?;
//             }
//         }
//         // Handle commas between objects
//         writeln!(
//             writer,
//             "{}",
//             if index < packages.len() - 1 { "," } else { "" }
//         )?;
//     }
//     writeln!(writer, "}}")?;

//     Ok(())
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::fs;

//     #[test]
//     fn test_map() {
//         let mut map = Map::new();
//         let info = Item {
//             path: "path".to_string(),
//             name: "name".to_string(),
//             outfile: "outfile".to_string(),
//             package: "package".to_string(),
//             lang: "lang".to_string(),
//             output_path: "output_path".to_string(),
//             bitrate: 128,
//             num_samples: 1000,
//             sample_rate: 44100,
//             modification_date: "2021-01-01".to_string(),
//         };
//         map.set("path".to_string(), info.clone());
//         println!("{:?}", map.value);
//         // assert_eq!(map.get("path"), Some(&info));
//     }

//     #[test]
//     fn test_map_from_vector() {
//         let info = Item {
//             path: "path".to_string(),
//             name: "name".to_string(),
//             outfile: "outfile".to_string(),
//             package: "package".to_string(),
//             lang: "lang".to_string(),
//             output_path: "output_path".to_string(),
//             bitrate: 128,
//             num_samples: 1000,
//             sample_rate: 44100,
//             modification_date: "2021-01-01".to_string(),
//         };
//         let vec = vec![info.clone()];
//         let map = Map::from_vec(vec);
//         println!("{:?}", map.value);
//         // assert_eq!(map.get("path"), Some(&info));
//     }

//     #[test]
//     fn test_write_to_json() {
//         let mut map = Map::new();
//         let info = Item {
//             path: "path".to_string(),
//             name: "name".to_string(),
//             outfile: "outfile".to_string(),
//             package: "package".to_string(),
//             lang: "lang".to_string(),
//             output_path: "output_path".to_string(),
//             bitrate: 128,
//             num_samples: 1000,
//             sample_rate: 44100,
//             modification_date: "2021-01-01".to_string(),
//         };
//         map.set("path".to_string(), info.clone());
//         map.save_cache_json().unwrap();
//         let file = fs::read_to_string(".cache/info.json").unwrap();
//         let expected = serde_json::to_string_pretty(&map.value).unwrap();
//         assert_eq!(file, expected);
//     }
// }
