//! What makes it open at once: every folder listing, the first photos, who's signed in and the
//! thumbnails are kept on disk (`~/.cache/tpcloud/app/<network>/`), shown straight away next
//! time, and refreshed underneath.

use crate::net::{Entry, Me, Photo};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

fn root(network: &str) -> PathBuf {
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("tpcloud/app").join(network)
}

fn key(text: &str) -> String {
    let mut h = DefaultHasher::new();
    text.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn write(path: PathBuf, bytes: &[u8]) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, bytes).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

pub fn listing(network: &str, path: &str) -> Option<Vec<Entry>> {
    std::fs::read(root(network).join("dirs").join(key(path))).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn save_listing(network: &str, path: &str, entries: &[Entry]) {
    if let Ok(json) = serde_json::to_vec(entries) {
        write(root(network).join("dirs").join(key(path)), &json);
    }
}

pub fn me(network: &str) -> Option<Me> {
    std::fs::read(root(network).join("me.json")).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn save_me(network: &str, me: &Me) {
    if let Ok(json) = serde_json::to_vec(me) {
        write(root(network).join("me.json"), &json);
    }
}

pub fn photos(network: &str) -> Option<Vec<Photo>> {
    std::fs::read(root(network).join("photos.json")).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn save_photos(network: &str, photos: &[Photo]) {
    if let Ok(json) = serde_json::to_vec(&photos[..photos.len().min(600)]) {
        write(root(network).join("photos.json"), &json);
    }
}

/// A thumbnail's file: keyed by the file's path, size and date, so a changed file gets a new one.
pub fn thumb_file(network: &str, id: &str) -> PathBuf {
    root(network).join("thumbs").join(format!("{}.jpg", key(id)))
}

pub fn thumb(network: &str, id: &str) -> Option<Vec<u8>> {
    std::fs::read(thumb_file(network, id)).ok()
}

pub fn save_thumb(network: &str, id: &str, bytes: &[u8]) {
    write(thumb_file(network, id), bytes);
}

/// Where files opened from the app are put (and found again, unchanged, the next time).
pub fn open_dir(network: &str) -> PathBuf {
    root(network).join("open")
}
