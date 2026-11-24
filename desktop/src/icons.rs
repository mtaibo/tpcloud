//! Icons: TPCloud's own line set (the same paths as `web/ui/Icon.vue`: 24-pt grid, round caps,
//! drawn to match SF Symbols' weight), tinted at draw time, plus the Finder-style folder and file
//! icons for the icon view. No emoji: fonts may not have them.

use iced::widget::svg;
use std::collections::HashMap;
use std::sync::OnceLock;

const LINE: &[(&str, &str)] = &[
    ("house", r#"<path d="M4 10.5 12 4l8 6.5V19a1.5 1.5 0 0 1-1.5 1.5H15v-5.5H9v5.5H5.5A1.5 1.5 0 0 1 4 19z"/>"#),
    ("folder", r#"<path d="M3.5 7.5A2 2 0 0 1 5.5 5.5h3.6l2 2h7.4a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>"#),
    ("person-2", r#"<circle cx="9" cy="8.5" r="3.3"/><path d="M2.8 19.5c.5-3.4 3.1-5.5 6.2-5.5s5.7 2.1 6.2 5.5"/><path d="M15.5 5.6a3.2 3.2 0 0 1 0 6.1M17.5 14.3c2 .7 3.4 2.5 3.7 5.2"/>"#),
    ("person", r#"<circle cx="12" cy="8" r="3.8"/><path d="M4.5 20c.6-3.9 3.7-6.3 7.5-6.3s6.9 2.4 7.5 6.3"/>"#),
    ("photo", r#"<rect x="3.5" y="5" width="17" height="14" rx="2.5"/><circle cx="9" cy="10" r="1.6"/><path d="m4.5 17.5 4.8-4.6 3.4 3.2 2.6-2.4 4.7 4.3"/>"#),
    ("gear", r#"<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2.2M12 18.3v2.2M20.5 12h-2.2M5.7 12H3.5M18 6l-1.6 1.6M7.6 16.4 6 18M18 18l-1.6-1.6M7.6 7.6 6 6"/>"#),
    ("cloud", r#"<path d="M7 18.5h10.5a4 4 0 0 0 .6-8 5.5 5.5 0 0 0-10.6-1.3A4.7 4.7 0 0 0 7 18.5z"/>"#),
    ("chevron-left", r#"<path d="M15 5.5 8.5 12 15 18.5"/>"#),
    ("chevron-right", r#"<path d="M9 5.5 15.5 12 9 18.5"/>"#),
    ("chevron-up", r#"<path d="M5.5 15 12 8.5 18.5 15"/>"#),
    ("chevron-down", r#"<path d="M5.5 9 12 15.5 18.5 9"/>"#),
    ("magnifyingglass", r#"<circle cx="10.5" cy="10.5" r="6"/><path d="m15 15 5 5"/>"#),
    ("download", r#"<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14"/>"#),
    ("upload", r#"<path d="M12 15.5V4.5M7.5 9 12 4.5 16.5 9M5 19.5h14"/>"#),
    ("list", r#"<path d="M9 6.5h11M9 12h11M9 17.5h11M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01"/>"#),
    ("grid", r#"<rect x="4" y="4" width="6.5" height="6.5" rx="1.6"/><rect x="13.5" y="4" width="6.5" height="6.5" rx="1.6"/><rect x="4" y="13.5" width="6.5" height="6.5" rx="1.6"/><rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1.6"/>"#),
    ("blinds", r#"<path d="M3.5 4h17"/><path d="M5 7.5h14M5 11h14M5 14.5h14"/><path d="M12 14.5v3.3"/><circle cx="12" cy="19.4" r="1.5"/>"#),
    ("sign-out", r#"<path d="M14.5 4.5H17a2.5 2.5 0 0 1 2.5 2.5v10a2.5 2.5 0 0 1-2.5 2.5h-2.5M10 8l-4 4 4 4M6 12h9.5"/>"#),
    ("server", r#"<rect x="4" y="4" width="16" height="7" rx="2"/><rect x="4" y="13" width="16" height="7" rx="2"/><path d="M8 7.5h.01M8 16.5h.01"/>"#),
    ("network", r#"<circle cx="12" cy="5.5" r="2"/><circle cx="5.5" cy="18.5" r="2"/><circle cx="18.5" cy="18.5" r="2"/><path d="M12 7.5v4M12 11.5l-5.2 5.4M12 11.5l5.2 5.4"/>"#),
    ("trash", r#"<path d="M4.5 7h15M9.5 7V4.8h5V7M6.5 7l.9 12.2A1.8 1.8 0 0 0 9.2 21h5.6a1.8 1.8 0 0 0 1.8-1.8L17.5 7M10 11v6M14 11v6"/>"#),
    ("plus", r#"<path d="M12 5v14M5 12h14"/>"#),
    ("refresh", r#"<path d="M19.5 12a7.5 7.5 0 1 1-2.2-5.3l2.2 2.1"/><path d="M19.5 4.5v4.3h-4.3"/>"#),
    ("info", r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.8h.01"/>"#),
    ("warning", r#"<path d="M12 4 21 19.5H3z"/><path d="M12 10v4.5M12 17h.01"/>"#),
    ("lock", r#"<rect x="5" y="10.5" width="14" height="10" rx="2.5"/><path d="M8 10.5V8a4 4 0 0 1 8 0v2.5"/>"#),
    ("stop", r#"<rect x="7.5" y="7.5" width="9" height="9" rx="2" fill="currentColor"/>"#),
    ("xmark", r#"<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>"#),
    ("checkmark", r#"<path d="M5 12.5l4.5 4.5L19 7.5"/>"#),
    ("paintbrush", r#"<path d="M14.5 4.5 19.5 9.5 11 18l-5-5z"/><path d="M6 13c-1.6.4-2.5 1.8-2.5 3.5 0 1.5-.5 2.5-1.5 3 3.5.5 6.8-.5 7.5-3.5"/>"#),
];

fn line_handles() -> &'static HashMap<&'static str, svg::Handle> {
    static H: OnceLock<HashMap<&'static str, svg::Handle>> = OnceLock::new();
    H.get_or_init(|| {
        LINE.iter()
            .map(|(name, paths)| {
                // Drawn black; iced tints it with the colour each use asks for.
                let doc = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round">{}</svg>"#, paths.replace("currentColor", "black"));
                (*name, svg::Handle::from_memory(doc.into_bytes()))
            })
            .collect()
    })
}

pub fn line(name: &str) -> svg::Handle {
    line_handles().get(name).cloned().unwrap_or_else(|| line_handles()["info"].clone())
}

/// The Finder's blue folder (light and dark keep the same blue, as on a Mac).
pub fn folder() -> svg::Handle {
    static H: OnceLock<svg::Handle> = OnceLock::new();
    H.get_or_init(|| {
        let doc = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 52">
<path d="M3 8.5A4.5 4.5 0 0 1 7.5 4h15.2c1.2 0 2.3.5 3.2 1.3L30 9.5h26.5A4.5 4.5 0 0 1 61 14v4H3z" fill="#2f8ff2"/>
<rect x="3" y="12" width="58" height="37" rx="5" fill="#5aaefb"/>
<rect x="3" y="12" width="58" height="2" rx="1" fill="#8cc8ff" opacity=".7"/>
</svg>"##;
        svg::Handle::from_memory(doc.as_bytes().to_vec())
    })
    .clone()
}

/// What kind of file, for its icon's colour and its label in the list.
pub fn kind_of(name: &str) -> (&'static str, &'static str) {
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "heic" | "heif" | "gif" | "webp" | "tif" | "tiff" | "bmp" | "avif" | "raw" | "dng" => ("image", "Image"),
        "mp4" | "mov" | "m4v" | "mkv" | "avi" | "webm" => ("video", "Movie"),
        "mp3" | "m4a" | "wav" | "flac" | "aac" | "ogg" | "opus" => ("audio", "Audio"),
        "pdf" => ("pdf", "PDF document"),
        "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "xz" | "zst" => ("archive", "Archive"),
        "txt" | "md" | "rtf" | "log" | "csv" => ("text", "Text"),
        "doc" | "docx" | "odt" | "pages" => ("doc", "Document"),
        "xls" | "xlsx" | "ods" | "numbers" => ("sheet", "Spreadsheet"),
        "ppt" | "pptx" | "odp" | "key" => ("slides", "Presentation"),
        "rs" | "py" | "js" | "ts" | "c" | "h" | "cpp" | "java" | "swift" | "go" | "sh" | "json" | "toml" | "yaml" | "yml" | "html" | "css" | "vue" => ("code", "Source code"),
        _ => ("file", "Document"),
    }
}

/// A page with a folded corner and the kind's colour band (and its extension), like the Finder's
/// generic document icons.
pub fn file(name: &str) -> svg::Handle {
    static H: OnceLock<std::sync::Mutex<HashMap<String, svg::Handle>>> = OnceLock::new();
    let ext: String = name.rsplit_once('.').map(|(_, e)| e.to_ascii_uppercase()).unwrap_or_default().chars().take(4).collect();
    let (kind, _) = kind_of(name);
    let key = format!("{kind}:{ext}");
    let cache = H.get_or_init(|| std::sync::Mutex::new(HashMap::new()));
    if let Some(h) = cache.lock().unwrap().get(&key) {
        return h.clone();
    }
    let band = match kind {
        "image" => "#34c759",
        "video" => "#af52de",
        "audio" => "#ff2d55",
        "pdf" => "#ff3b30",
        "archive" => "#8e8e93",
        "text" => "#8e8e93",
        "doc" => "#007aff",
        "sheet" => "#34c759",
        "slides" => "#ff9500",
        "code" => "#5856d6",
        _ => "#aeaeb2",
    };
    // No <text> in the SVG: iced's renderer draws it without fonts. The band's colour says the kind.
    let label = String::new();
    let doc = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 52 64">
<path d="M8 2h25l15 15v41a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4V6a4 4 0 0 1 4-4z" fill="#fdfdfd" stroke="#d1d1d6" stroke-width="1.2"/>
<path d="M33 2v11a4 4 0 0 0 4 4h11" fill="#ececf0" stroke="#d1d1d6" stroke-width="1.2" stroke-linejoin="round"/>
<rect x="9" y="41" width="34" height="12" rx="3" fill="{band}"/>{label}
</svg>"##
    );
    let h = svg::Handle::from_memory(doc.into_bytes());
    cache.lock().unwrap().insert(key, h.clone());
    h
}
