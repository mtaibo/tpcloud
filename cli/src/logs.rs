//! Reading the processes' logs (logs/<name>.log), and following them.

use crate::paths;
use anyhow::{bail, Result};
use std::io::{Read, Seek, SeekFrom, Write};

pub const NAMES: [&str; 6] = ["gateway", "auth", "store-api", "home-api", "mqtt-broker", "cloudflared"];

/// The last `n` lines of a log.
pub fn tail(name: &str, n: usize) -> Vec<String> {
    let path = paths::logs_dir().join(format!("{name}.log"));
    let Ok(text) = std::fs::read(&path) else { return Vec::new() };
    let text = String::from_utf8_lossy(&text);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect()
}

pub fn show(services: &[String], n: usize, follow: bool) -> Result<()> {
    let names: Vec<String> = if services.is_empty() {
        NAMES.iter().filter(|n| paths::logs_dir().join(format!("{n}.log")).exists()).map(|s| s.to_string()).collect()
    } else {
        services.to_vec()
    };
    if names.is_empty() {
        bail!("no logs yet in {}", paths::logs_dir().display());
    }
    for name in &names {
        if names.len() > 1 {
            println!("── {name}");
        }
        for line in tail(name, n) {
            println!("{line}");
        }
    }
    if follow {
        let name = names.last().unwrap();
        let path = paths::logs_dir().join(format!("{name}.log"));
        let mut file = std::fs::File::open(&path)?;
        let mut pos = file.seek(SeekFrom::End(0))?;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(400));
            let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if len < pos {
                // Rotated: start from the new file's beginning.
                file = std::fs::File::open(&path)?;
                pos = 0;
            }
            if len > pos {
                file.seek(SeekFrom::Start(pos))?;
                let mut chunk = Vec::new();
                file.read_to_end(&mut chunk)?;
                pos += chunk.len() as u64;
                std::io::stdout().write_all(&chunk)?;
                std::io::stdout().flush()?;
            }
        }
    }
    Ok(())
}
