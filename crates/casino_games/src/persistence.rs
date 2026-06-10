//! Saving and loading the player's profile so progress carries between sessions.
//!
//! The profile (name, chip balance, and basic lifetime stats) is stored as JSON
//! in the platform's standard data directory, e.g. `~/.local/share/casino/` on
//! Linux. The schema is versioned so it can grow without breaking old saves.

use std::fs::{self, File};
use std::path::PathBuf;

use chrono::Local;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

const PROFILE_VERSION: u32 = 1;

/// A persisted player profile.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Profile {
    /// Schema version, for forward-compatible migrations.
    pub version: u32,
    pub name: String,
    pub chips: u32,
    pub hands_played: u64,
    /// Hands where the player ended chip-positive (won a pot net of their bets).
    pub hands_won: u64,
    /// Render cards as Unicode playing-card glyphs (`🂡`) instead of text (`As`).
    /// Defaults to `false` so older saves load fine.
    #[serde(default)]
    pub glyph_cards: bool,
}

impl Profile {
    pub fn new(name: &str, chips: u32) -> Self {
        Self {
            version: PROFILE_VERSION,
            name: name.to_string(),
            chips,
            hands_played: 0,
            hands_won: 0,
            glyph_cards: false,
        }
    }
}

fn profile_path() -> Option<PathBuf> {
    let dirs = ProjectDirs::from("com", "winstoncooke", "casino")?;
    Some(dirs.data_dir().join("profile.json"))
}

/// Loads the saved profile, or `None` if there is no save, it is unreadable, or
/// it is from a newer (unsupported) schema version.
pub fn load() -> Option<Profile> {
    let path = profile_path()?;
    let contents = fs::read_to_string(path).ok()?;
    let profile: Profile = serde_json::from_str(&contents).ok()?;
    if profile.version > PROFILE_VERSION {
        eprintln!(
            "Warning: saved profile is version {} but this build supports up to {}; ignoring it.",
            profile.version, PROFILE_VERSION
        );
        return None;
    }
    Some(profile)
}

/// Saves the profile, creating the data directory if needed. The write is atomic
/// (temp file + rename) so a crash mid-write can't corrupt an existing save.
/// Failures are reported but non-fatal — losing a save shouldn't crash the game.
pub fn save(profile: &Profile) {
    let Some(path) = profile_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("Warning: couldn't create the save directory: {err}");
            return;
        }
    }
    let json = match serde_json::to_string_pretty(profile) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("Warning: couldn't serialize your progress: {err}");
            return;
        }
    };
    // Write to a sibling temp file, then atomically rename over the target.
    let tmp = path.with_extension("json.tmp");
    if let Err(err) = fs::write(&tmp, &json) {
        eprintln!("Warning: couldn't save your progress: {err}");
        return;
    }
    if let Err(err) = fs::rename(&tmp, &path) {
        eprintln!("Warning: couldn't finalize your save: {err}");
        let _ = fs::remove_file(&tmp);
    }
}

/// A human-readable description of where the profile is stored, for display.
pub fn save_location() -> String {
    profile_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "an unknown location".to_string())
}

/// Creates a fresh timestamped hand-history file for this session under the data
/// dir's `history/` subdirectory, e.g. `…/history/2026-06-10_14-32-05.txt`, and
/// returns the open file and its path.
///
/// One file per session keeps each log small and self-limiting (old sessions are
/// separate files you can prune). Returns `None` if the directory or file can't
/// be created — non-fatal, the game still streams the history to stdout.
pub fn new_session_history() -> Option<(File, PathBuf)> {
    let dir = ProjectDirs::from("com", "winstoncooke", "casino")?
        .data_dir()
        .join("history");
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.txt", Local::now().format("%Y-%m-%d_%H-%M-%S")));
    let file = File::create(&path).ok()?;
    Some((file, path))
}
