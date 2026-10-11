//! Reads the game's journal folder: replays the newest journal from its first line,
//! then follows appended lines, and re-reads companion JSON files when they change.

use serde_json::Value;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Snapshot files the game overwrites in place. Each is a JSON object with an "event" field.
pub const COMPANION_FILES: [&str; 7] = [
    "Status.json",
    "Cargo.json",
    "NavRoute.json",
    "Market.json",
    "ModulesInfo.json",
    "ShipLocker.json",
    "Backpack.json",
];

/// `<Saved Games>\Frontier Developments\Elite Dangerous`, resolved through the Known Folder API
/// because users can move Saved Games.
pub fn default_dir() -> Option<PathBuf> {
    saved_games().map(|p| p.join("Frontier Developments").join("Elite Dangerous"))
}

#[cfg(windows)]
fn saved_games() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_SavedGames, SHGetKnownFolderPath};

    let mut raw: *mut u16 = std::ptr::null_mut();
    // SAFETY: valid GUID pointer and out-pointer; the returned buffer is freed with CoTaskMemFree.
    unsafe {
        let hr = SHGetKnownFolderPath(&FOLDERID_SavedGames, 0, std::ptr::null_mut(), &mut raw);
        let path = if hr >= 0 && !raw.is_null() {
            let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
            Some(PathBuf::from(std::ffi::OsString::from_wide(std::slice::from_raw_parts(raw, len))))
        } else {
            None
        };
        CoTaskMemFree(raw as *const _);
        path
    }
}

#[cfg(not(windows))]
fn saved_games() -> Option<PathBuf> {
    None
}

/// Newest `Journal.*.log` by modified time. Names come in two date formats, so never sort by name.
pub fn newest_journal(dir: &Path) -> Option<PathBuf> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_ascii_lowercase();
            n.starts_with("journal.") && n.ends_with(".log")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

pub struct Reader {
    dir: PathBuf,
    journal: Option<PathBuf>,
    offset: u64,
    partial: Vec<u8>,
    seen: HashMap<&'static str, SystemTime>,
}

impl Reader {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, journal: None, offset: 0, partial: Vec::new(), seen: HashMap::new() }
    }

    /// New events since the last call: journal lines first, then any changed companion files.
    pub fn poll(&mut self) -> Vec<Value> {
        let mut out = Vec::new();
        if let Some(newest) = newest_journal(&self.dir) {
            if self.journal.as_ref() != Some(&newest) {
                // New session or a `Continued` part file: replay it from the top.
                self.journal = Some(newest);
                self.offset = 0;
                self.partial.clear();
            }
            self.read_journal(&mut out);
        }
        for name in COMPANION_FILES {
            self.read_companion(name, &mut out);
        }
        out
    }

    fn read_journal(&mut self, out: &mut Vec<Value>) {
        let Some(path) = &self.journal else { return };
        // std opens with shared read/write/delete access, so the game can keep writing.
        let Ok(mut file) = File::open(path) else { return };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            self.offset = 0;
            self.partial.clear();
        }
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        let Ok(n) = file.read_to_end(&mut buf) else { return };
        self.offset += n as u64;
        self.partial.extend_from_slice(&buf);

        // Only complete lines; a read can stop mid-line while the game is writing.
        let Some(end) = self.partial.iter().rposition(|&b| b == b'\n') else { return };
        let complete: Vec<u8> = self.partial.drain(..=end).collect();
        for line in complete.split(|&b| b == b'\n') {
            let line = String::from_utf8_lossy(line);
            let line = line.trim();
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                out.push(v);
            }
        }
    }

    fn read_companion(&mut self, name: &'static str, out: &mut Vec<Value>) {
        let path = self.dir.join(name);
        let Ok(modified) = fs::metadata(&path).and_then(|m| m.modified()) else { return };
        if self.seen.get(name) == Some(&modified) {
            return;
        }
        // A half-written file fails to parse; leaving `seen` unchanged retries it on the next poll.
        let Ok(text) = fs::read_to_string(&path) else { return };
        if let Ok(v) = serde_json::from_str::<Value>(&text) {
            self.seen.insert(name, modified);
            out.push(v);
        }
    }
}
