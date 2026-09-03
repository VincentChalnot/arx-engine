//! Persistent user preferences: last-used AI difficulty, display toggles,
//! and whether the first-launch help modal has already been shown. Lives at
//! `~/.config/keres/settings.bin` on Linux (via `dirs_next::config_dir()`),
//! entirely separate from the per-game autosaves in `crate::save`.
//!
//! Format (`[u8 version][u8 level][u8 flags]`) is versioned the same way as
//! `crate::save`'s save-file format: a corrupt or foreign-version file is
//! silently ignored in favor of defaults rather than misread.

use std::io::{Read, Write};
use std::path::PathBuf;

const FORMAT_VERSION: u8 = 1;
const FLAG_SHOW_COORDS: u8 = 1 << 0;
const FLAG_SHOW_THREATS: u8 = 1 << 1;
const FLAG_HELP_SEEN: u8 = 1 << 2;
const FLAG_ROTATE_ICONS: u8 = 1 << 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    pub level: u8,
    pub show_coords: bool,
    pub show_threats: bool,
    /// Whether the first-launch mini rules modal has already been shown —
    /// once true, `App::start_game` never shows it again.
    pub help_seen: bool,
    /// Whether the far side's piece icons render upside down (see
    /// `main.rs::is_upside_down`). Off by default — most players find
    /// upright icons easier to read on a screen than the physical-board
    /// convention it mimics.
    pub rotate_opponent_icons: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            // A gentle default for players who haven't picked a difficulty yet.
            level: 3,
            show_coords: true,
            show_threats: true,
            help_seen: false,
            rotate_opponent_icons: false,
        }
    }
}

thread_local! {
    static DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Pin this thread's settings folder to an isolated directory instead of the
/// real one — used by tests and the `KERES_SNAPSHOT` headless render path,
/// same rationale as `crate::save::set_test_dir_override`.
pub fn set_test_dir_override(p: PathBuf) {
    DIR_OVERRIDE.with(|o| *o.borrow_mut() = Some(p));
}

fn settings_path() -> PathBuf {
    if let Some(p) = DIR_OVERRIDE.with(|o| o.borrow().clone()) {
        return p.join("settings.bin");
    }
    dirs_next::config_dir()
        .map(|d| d.join("keres").join("settings.bin"))
        .unwrap_or_else(|| PathBuf::from("keres_settings.bin"))
        .to_path_buf()
}

pub fn load() -> Settings {
    let Ok(mut f) = std::fs::File::open(settings_path()) else {
        return Settings::default();
    };
    let mut bytes = Vec::new();
    if f.read_to_end(&mut bytes).is_err() || bytes.len() != 3 || bytes[0] != FORMAT_VERSION {
        return Settings::default();
    }
    let level = bytes[1];
    if !(keres_engine::engine::constants::MIN_LEVEL..=keres_engine::engine::constants::MAX_LEVEL)
        .contains(&level)
    {
        return Settings::default();
    }
    let flags = bytes[2];
    Settings {
        level,
        show_coords: flags & FLAG_SHOW_COORDS != 0,
        show_threats: flags & FLAG_SHOW_THREATS != 0,
        help_seen: flags & FLAG_HELP_SEEN != 0,
        rotate_opponent_icons: flags & FLAG_ROTATE_ICONS != 0,
    }
}

pub fn save(settings: &Settings) {
    let mut flags = 0u8;
    if settings.show_coords {
        flags |= FLAG_SHOW_COORDS;
    }
    if settings.show_threats {
        flags |= FLAG_SHOW_THREATS;
    }
    if settings.help_seen {
        flags |= FLAG_HELP_SEEN;
    }
    if settings.rotate_opponent_icons {
        flags |= FLAG_ROTATE_ICONS;
    }
    let bytes = [FORMAT_VERSION, settings.level, flags];
    let path = settings_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = f.write_all(&bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        set_test_dir_override(std::env::temp_dir().join("keres_test_settings_missing"));
        let _ = std::fs::remove_file(settings_path());
        assert_eq!(load(), Settings::default());
    }

    #[test]
    fn save_load_roundtrip() {
        set_test_dir_override(std::env::temp_dir().join("keres_test_settings_roundtrip"));
        let settings = Settings {
            level: 4,
            show_coords: false,
            show_threats: true,
            help_seen: true,
            rotate_opponent_icons: true,
        };
        save(&settings);
        assert_eq!(load(), settings);
    }

    #[test]
    fn corrupt_file_yields_defaults_not_a_panic() {
        set_test_dir_override(std::env::temp_dir().join("keres_test_settings_corrupt"));
        let path = settings_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&path, [9u8, 200, 0]).unwrap();
        assert_eq!(load(), Settings::default());
    }
}
