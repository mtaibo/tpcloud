//! Reading and changing `KEY=value` settings files without touching anything else in them.

use anyhow::{Context, Result};
use std::path::Path;

/// The value of `key`, without a trailing `# comment`, if the file sets it.
pub fn get(path: &Path, key: &str) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let prefix = format!("{key}=");
    text.lines()
        .filter(|line| line.starts_with(&prefix))
        .last()
        .map(|line| strip_comment(&line[prefix.len()..]).to_string())
}

pub fn strip_comment(value: &str) -> &str {
    // Compose's own rule: an inline comment needs a space before the #.
    match value.find(" #") {
        Some(i) => value[..i].trim_end(),
        None => value.trim_end(),
    }
}

/// Replaces the key's line, or adds it at the end, leaving every other line alone.
pub fn set(path: &Path, key: &str, value: &str) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let prefix = format!("{key}=");
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with(&prefix) {
                found = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("{key}={value}"));
    }
    std::fs::write(path, lines.join("\n") + "\n").with_context(|| format!("writing {}", path.display()))
}

/// A comma-separated list setting (TPCLOUD_MODULES, COMPOSE_PROFILES).
pub fn get_list(path: &Path, key: &str) -> Vec<String> {
    get(path, key)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn set_list(path: &Path, key: &str, items: &[String]) -> Result<()> {
    set(path, key, &items.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_edits_without_touching_other_lines() {
        let dir = std::env::temp_dir().join(format!("tpcloud-env-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.env");
        std::fs::write(&path, "# a comment\nTPCLOUD_URL=https://x.example   # the address\nTPCLOUD_MODULES=store\n").unwrap();
        assert_eq!(get(&path, "TPCLOUD_URL").as_deref(), Some("https://x.example"));
        assert_eq!(get_list(&path, "TPCLOUD_MODULES"), vec!["store"]);
        set_list(&path, "TPCLOUD_MODULES", &["store".into(), "home".into()]).unwrap();
        set(&path, "NEW_KEY", "1").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text, "# a comment\nTPCLOUD_URL=https://x.example   # the address\nTPCLOUD_MODULES=store,home\nNEW_KEY=1\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
