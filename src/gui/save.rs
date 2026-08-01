//! Autosave/resume: a tiny on-disk format so a game in progress survives
//! quitting the app. Stored next to the executable as `keres_save.bin`.
//! Format: [mode: u8][move_count: u16 LE][move: u16 LE; move_count].

use crate::app::Mode;
use keres_engine::Move;
use std::io::{Read, Write};
use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    static TEST_PATH_OVERRIDE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Test-only: pin this thread's save path to an isolated file so parallel
/// tests never race on the real `keres_save.bin` next to the executable.
#[cfg(test)]
pub fn set_test_path_override(p: PathBuf) {
    TEST_PATH_OVERRIDE.with(|o| *o.borrow_mut() = Some(p));
}

fn save_path() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(p) = TEST_PATH_OVERRIDE.with(|o| o.borrow().clone()) {
            return p;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("keres_save.bin")))
        .unwrap_or_else(|| PathBuf::from("keres_save.bin"))
}

fn mode_byte(mode: Mode) -> u8 {
    match mode {
        Mode::Hotseat => 0,
        Mode::VsAiWhite => 1,
        Mode::VsAiBlack => 2,
    }
}

fn mode_from_byte(b: u8) -> Option<Mode> {
    match b {
        0 => Some(Mode::Hotseat),
        1 => Some(Mode::VsAiWhite),
        2 => Some(Mode::VsAiBlack),
        _ => None,
    }
}

pub fn save(mode: Mode, moves: &[Move]) {
    let mut bytes = Vec::with_capacity(3 + moves.len() * 2);
    bytes.push(mode_byte(mode));
    bytes.extend_from_slice(&(moves.len() as u16).to_le_bytes());
    for mv in moves {
        bytes.extend_from_slice(&mv.to_u16().to_le_bytes());
    }
    if let Ok(mut f) = std::fs::File::create(save_path()) {
        let _ = f.write_all(&bytes);
    }
}

pub fn clear() {
    let _ = std::fs::remove_file(save_path());
}

pub fn exists() -> bool {
    save_path().is_file()
}

pub fn load() -> Option<(Mode, Vec<Move>)> {
    let mut f = std::fs::File::open(save_path()).ok()?;
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).ok()?;
    if bytes.len() < 3 {
        return None;
    }
    let mode = mode_from_byte(bytes[0])?;
    let count = u16::from_le_bytes([bytes[1], bytes[2]]) as usize;
    if bytes.len() != 3 + count * 2 {
        return None;
    }
    let mut moves = Vec::with_capacity(count);
    for i in 0..count {
        let off = 3 + i * 2;
        moves.push(Move::from_u16(u16::from_le_bytes([
            bytes[off],
            bytes[off + 1],
        ])));
    }
    Some((mode, moves))
}

#[cfg(test)]
mod tests {
    use super::*;
    use keres_engine::Position;

    #[test]
    fn save_load_roundtrip() {
        set_test_path_override(std::env::temp_dir().join("keres_test_save_roundtrip.bin"));
        let moves = vec![
            Move {
                from: Position::new(2, 6),
                to: Position::new(3, 5),
                unstack: false,
            },
            Move {
                from: Position::new(6, 1),
                to: Position::new(6, 4),
                unstack: true,
            },
        ];
        save(Mode::VsAiBlack, &moves);
        let (mode, loaded) = load().expect("save should be readable back");
        assert_eq!(mode, Mode::VsAiBlack);
        assert_eq!(loaded, moves);
        clear();
        assert!(!exists());
    }
}
