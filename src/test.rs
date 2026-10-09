
#[test]
fn siphash_matches_old_default_hasher_naming() {
    use crate::{config::HashAlgorithm, hash_buffer};
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    let buffer = std::fs::read("example/packages/template/music_drums.wav").unwrap();
    // the naming used before the switch to sha256
    let mut hasher = DefaultHasher::new();
    buffer.hash(&mut hasher);
    let old = hasher.finish().to_string();
    let old = &old[..15];
    assert_eq!(hash_buffer(&buffer, HashAlgorithm::Siphash), old);
}

#[test]
fn sha256_naming() {
    use crate::{config::HashAlgorithm, hash_buffer};
    let hash = hash_buffer(&b"hello".to_vec(), HashAlgorithm::Sha256);
    assert_eq!(hash, "2cf24dba5f");
}

#[test]
fn mp4_source_setting_overrides_package_setting() {
    use crate::{
        config::{Package, Source},
        source_includes_mp4,
    };
    let package = |include_mp4| Package {
        sourcedir: None,
        bitrate: None,
        extends: None,
        languages: None,
        sources: None,
        include_flac: None,
        include_mp4,
    };
    let source = |include_mp4| Source {
        bitrate: None,
        channels: None,
        include_mp4,
    };
    assert!(!source_includes_mp4(None, &package(None)));
    assert!(source_includes_mp4(None, &package(Some(true))));
    assert!(source_includes_mp4(Some(&source(Some(true))), &package(None)));
    assert!(!source_includes_mp4(Some(&source(Some(false))), &package(Some(true))));
    assert!(source_includes_mp4(Some(&source(None)), &package(Some(true))));
}
