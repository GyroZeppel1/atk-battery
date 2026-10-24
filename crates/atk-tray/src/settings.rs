//! Tiny INI settings store, no dependency.
//!
//! Kept as a plain text file under LocalAppData so the user can read and edit
//! it, and so nothing is hidden in the registry.

use std::path::PathBuf;

#[derive(Clone, Copy)]
pub struct Settings {
    /// Show the panel when launched by hand. Login starts always stay in the
    /// tray regardless of this flag.
    pub show_panel: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { show_panel: true }
    }
}

fn dir() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    let mut p = PathBuf::from(base);
    p.push("ATK Battery");
    Some(p)
}

pub fn path() -> Option<PathBuf> {
    dir().map(|d| d.join("settings.ini"))
}

pub fn load() -> Settings {
    let mut s = Settings::default();
    let Some(p) = path() else {
        return s;
    };
    let Ok(text) = std::fs::read_to_string(p) else {
        return s;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "show_panel" {
                s.show_panel = matches!(v.trim(), "1" | "true" | "yes" | "on");
            }
        }
    }
    s
}

pub fn save(s: &Settings) {
    let Some(p) = path() else {
        return;
    };
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let body = format!(
        "# ATK Battery settings\n# show_panel: 1 = launch with the panel open, 0 = tray only\nshow_panel={}\n",
        if s.show_panel { 1 } else { 0 }
    );
    let _ = std::fs::write(p, body);
}
