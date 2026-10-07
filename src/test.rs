
#[test]
fn te() {
    let mut x = 1_i32;
    println!("{x}");
    x.saturating_sub(1);
    println!("{x}");
}

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
