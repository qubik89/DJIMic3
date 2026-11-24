//! Finding transmitters among the mounted drives and listing their takes.
//!
//! A DJI Mic 3 transmitter in its charging case (or on the magnetic cable)
//! shows up as a FAT32 USB drive labelled `NO NAME`. Its recordings sit in a
//! folder at the root named `TX_MIC<nnn>_<YYYYMMDD>_<hhmmss>`, as
//! `TX<slot>_MIC<nnn>_<YYYYMMDD>_<hhmmss>_<orig|edit>.wav`. The label is
//! useless, so a drive counts as a transmitter when it has such a folder.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use serde::Serialize;

use crate::wav::{self, WavInfo};

/// Which of a take's files this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    /// `_orig`: the unprocessed track.
    Original,
    /// `_edit`: the noise-reduced copy written with dual-file recording.
    Processed,
    /// A WAV whose name doesn't follow the pattern.
    Other,
}

/// One WAV file on a transmitter.
#[derive(Debug, Clone, Serialize)]
pub struct RecordingFile {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileKind,
    pub size: u64,
    /// Already in the import folder with the same size.
    pub imported: bool,
}

/// One recording: the original file plus, with dual-file recording, the
/// processed copy taken at the same time.
#[derive(Debug, Clone, Serialize)]
pub struct Take {
    /// File name without the `_orig`/`_edit` suffix and extension.
    pub key: String,
    /// Receiver channel the transmitter was paired to (`TX01` → 1).
    pub slot: Option<u8>,
    /// Running take number on the transmitter (`MIC003` → 3).
    pub number: Option<u32>,
    /// Local start time, `YYYY-MM-DDThh:mm:ss`.
    pub recorded_at: Option<String>,
    /// Header details of the first readable file, if any.
    pub info: Option<WavInfo>,
    pub files: Vec<RecordingFile>,
}

impl Take {
    /// Day folder the take is imported into.
    pub fn day(&self) -> &str {
        self.recorded_at.as_deref().map(|s| &s[..10]).unwrap_or("undated")
    }
}

/// A transmitter drive and what's on it.
#[derive(Debug, Clone, Serialize)]
pub struct Transmitter {
    /// Stable for a given transmitter until its storage is formatted: the
    /// name of its first recordings folder.
    pub id: String,
    /// Where the drive is mounted.
    pub mount: PathBuf,
    /// Receiver channel its recordings were made on.
    pub slot: Option<u8>,
    /// Model from the newest take, e.g. `MIC 3`.
    pub model: Option<String>,
    /// Firmware from the newest take, e.g. `03.01.00.10`.
    pub firmware: Option<String>,
    /// Reached over its own Wi-Fi access point rather than as a USB drive.
    pub wireless: bool,
    /// Newest first.
    pub takes: Vec<Take>,
}

/// Parts of a transmitter file name.
#[derive(Debug, PartialEq)]
struct NameParts {
    slot: u8,
    number: u32,
    recorded_at: String,
    kind: FileKind,
    key: String,
}

fn digits(s: &str, n: usize) -> Option<&str> {
    (s.len() == n && s.bytes().all(|b| b.is_ascii_digit())).then_some(s)
}

/// `TX01_MIC003_20261123_232008_orig` → parts.
fn parse_name(stem: &str) -> Option<NameParts> {
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() < 4 {
        return None;
    }
    let slot = digits(parts[0].strip_prefix("TX")?, 2)?.parse().ok()?;
    let number = parts[1].strip_prefix("MIC")?;
    let number = (number.len() >= 3 && number.bytes().all(|b| b.is_ascii_digit())).then_some(number)?;
    let d = digits(parts[2], 8)?;
    let t = digits(parts[3], 6)?;
    let kind = match parts.get(4).copied() {
        None => FileKind::Original,
        Some(s) if s.eq_ignore_ascii_case("orig") => FileKind::Original,
        Some(s) if s.eq_ignore_ascii_case("edit") => FileKind::Processed,
        Some(_) => return None,
    };
    if parts.len() > 5 {
        return None;
    }
    Some(NameParts {
        slot,
        number: number.parse().ok()?,
        recorded_at: format!("{}-{}-{}T{}:{}:{}", &d[0..4], &d[4..6], &d[6..8], &t[0..2], &t[2..4], &t[4..6]),
        kind,
        key: parts[..4].join("_"),
    })
}

/// `TX_MIC002_20261117_121733`.
fn is_recordings_folder(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("TX_MIC") else { return false };
    let parts: Vec<&str> = rest.split('_').collect();
    parts.len() == 3 && digits(parts[0], 3).is_some() && digits(parts[1], 8).is_some() && digits(parts[2], 6).is_some()
}

fn is_wav(name: &str) -> bool {
    !name.starts_with('.') && name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(".wav")
}

/// Folders removable drives get mounted under on this OS.
pub fn mount_points() -> Vec<PathBuf> {
    // Skip symlinks: on macOS `/Volumes/Macintosh HD` points at `/`.
    fn children(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    out.push(e.path());
                }
            }
        }
    }
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        children(Path::new("/Volumes"), &mut out);
    } else if cfg!(windows) {
        for letter in b'C'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            if root.exists() {
                out.push(root);
            }
        }
    } else {
        // udisks: /media/<user>/<label> or /run/media/<user>/<label>; plain /media/<label> and /mnt/<label>.
        for base in ["/media", "/run/media"] {
            let mut users = Vec::new();
            children(Path::new(base), &mut users);
            for dir in users {
                children(&dir, &mut out);
                out.push(dir);
            }
        }
        children(Path::new("/mnt"), &mut out);
    }
    out
}

/// One WAV on a transmitter, before files are grouped into takes.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    /// Where to fetch it: a local path for a USB drive, the server path
    /// (`data/emmc0/…`) for a transmitter reached over Wi-Fi.
    pub path: PathBuf,
    pub size: u64,
    pub info: Option<WavInfo>,
}

/// Group a transmitter's files into takes (original + processed copy).
pub fn build_transmitter(id: String, mount: PathBuf, entries: Vec<Entry>) -> Transmitter {
    let mut takes: HashMap<String, Take> = HashMap::new();
    for Entry { name, path, size, info } in entries {
        let stem = &name[..name.len().saturating_sub(4)];
        let parts = parse_name(stem);
        let (key, kind) = match &parts {
            Some(p) => (p.key.clone(), p.kind),
            None => (stem.to_string(), FileKind::Other),
        };
        let take = takes.entry(key.clone()).or_insert_with(|| Take {
            key,
            slot: parts.as_ref().map(|p| p.slot),
            number: parts.as_ref().map(|p| p.number),
            recorded_at: parts.as_ref().map(|p| p.recorded_at.clone()).or_else(|| {
                info.as_ref()
                    .and_then(|i| i.origination.as_ref())
                    .filter(|(d, t)| d.len() == 10 && t.len() == 8)
                    .map(|(d, t)| format!("{d}T{t}"))
            }),
            info: None,
            files: Vec::new(),
        });
        // Prefer the original's header for the take.
        if take.info.is_none() || (kind == FileKind::Original && info.is_some()) {
            take.info = info.or(take.info.take());
        }
        take.files.push(RecordingFile { name, path, kind, size, imported: false });
    }

    let mut takes: Vec<Take> = takes.into_values().collect();
    for t in &mut takes {
        t.files.sort_by_key(|f| (f.kind != FileKind::Original, f.name.clone()));
    }
    takes.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at).then_with(|| b.key.cmp(&a.key)));

    // The channel most takes were recorded on.
    let mut counts: HashMap<u8, usize> = HashMap::new();
    for t in &takes {
        if let Some(s) = t.slot {
            *counts.entry(s).or_default() += 1;
        }
    }
    let slot = counts.into_iter().max_by_key(|&(s, n)| (n, std::cmp::Reverse(s))).map(|(s, _)| s);
    let newest = takes.iter().find_map(|t| t.info.as_ref());

    Transmitter {
        id,
        mount,
        slot,
        model: newest.and_then(|i| i.originator.clone()),
        firmware: newest
            .and_then(|i| i.description.as_deref())
            .map(|d| d.strip_prefix("ver:").unwrap_or(d).to_string()),
        wireless: false,
        takes,
    }
}

type CacheEntry = (u64, Option<SystemTime>, Option<WavInfo>);

/// Lists transmitters, remembering WAV headers between scans so a poll only
/// reads new or changed files.
#[derive(Default)]
pub struct Scanner {
    cache: Mutex<HashMap<PathBuf, CacheEntry>>,
}

impl Scanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every transmitter drive currently mounted.
    pub fn scan(&self) -> Vec<Transmitter> {
        self.scan_roots(&mount_points())
    }

    /// Transmitters among the given drive roots.
    pub fn scan_roots(&self, roots: &[PathBuf]) -> Vec<Transmitter> {
        let mut out: Vec<Transmitter> = roots.iter().filter_map(|r| self.scan_drive(r)).collect();
        out.sort_by(|a, b| a.slot.cmp(&b.slot).then_with(|| a.id.cmp(&b.id)));
        out
    }

    /// The transmitter on the drive at `root`, if it is one.
    pub fn scan_drive(&self, root: &Path) -> Option<Transmitter> {
        let mut folders: Vec<PathBuf> = fs::read_dir(root)
            .ok()?
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter(|e| is_recordings_folder(&e.file_name().to_string_lossy()))
            .map(|e| e.path())
            .collect();
        if folders.is_empty() {
            return None;
        }
        folders.sort();

        let mut entries = Vec::new();
        for folder in &folders {
            let Ok(rd) = fs::read_dir(folder) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if !is_wav(&name) {
                    continue;
                }
                let Ok(meta) = e.metadata() else { continue };
                if !meta.is_file() {
                    continue;
                }
                let path = e.path();
                let info = self.info(&path, meta.len(), meta.modified().ok());
                entries.push(Entry { name, path, size: meta.len(), info });
            }
        }
        let id = folders[0].file_name()?.to_string_lossy().into_owned();
        Some(build_transmitter(id, root.to_path_buf(), entries))
    }

    fn info(&self, path: &Path, size: u64, modified: Option<SystemTime>) -> Option<WavInfo> {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((s, m, info)) = cache.get(path) {
            if *s == size && *m == modified {
                return info.clone();
            }
        }
        let info = wav::read(path).ok();
        cache.insert(path.to_path_buf(), (size, modified, info.clone()));
        info
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TestDir;
    use crate::wav::tests::mic3_wav;

    #[test]
    fn parses_file_names() {
        let p = parse_name("TX01_MIC003_20261123_232008_edit").unwrap();
        assert_eq!(p.slot, 1);
        assert_eq!(p.number, 3);
        assert_eq!(p.recorded_at, "2026-11-23T23:20:08");
        assert_eq!(p.kind, FileKind::Processed);
        assert_eq!(p.key, "TX01_MIC003_20261123_232008");
        assert_eq!(parse_name("TX02_MIC001_20261117_114545_orig").unwrap().kind, FileKind::Original);
        assert_eq!(parse_name("TX02_MIC001_20261117_114545").unwrap().kind, FileKind::Original);
        assert!(parse_name("TX2_MIC001_20261117_114545_orig").is_none());
        assert!(parse_name("TX02_MIC001_2026111_114545_orig").is_none());
        assert!(parse_name("TX02_MIC001_20261117_114545_mix").is_none());
        assert!(parse_name("holiday").is_none());
    }

    #[test]
    fn recognises_recordings_folders() {
        assert!(is_recordings_folder("TX_MIC002_20261117_121733"));
        assert!(!is_recordings_folder("TX_MIC002"));
        assert!(!is_recordings_folder("DCIM"));
    }

    #[test]
    fn lists_a_transmitter_drive() {
        let dir = TestDir::new("scan");
        let drive = dir.path().join("NO NAME");
        let folder = drive.join("TX_MIC002_20261117_121733");
        fs::create_dir_all(&folder).unwrap();
        let older = mic3_wav(1, 24, 4800, "2026-11-17", "12:17:33");
        let newer = mic3_wav(1, 24, 48_000, "2026-11-23", "23:20:08");
        fs::write(folder.join("TX01_MIC001_20261117_121733_orig.wav"), &older).unwrap();
        fs::write(folder.join("TX01_MIC003_20261123_232008_orig.wav"), &newer).unwrap();
        fs::write(folder.join("TX01_MIC003_20261123_232008_edit.wav"), &newer).unwrap();
        fs::write(folder.join("._TX01_MIC003_20261123_232008_orig.wav"), b"apple double").unwrap();
        fs::write(folder.join("notes.txt"), b"x").unwrap();
        // A drive that isn't a transmitter.
        fs::create_dir_all(dir.path().join("USB").join("DCIM")).unwrap();

        let scanner = Scanner::new();
        let txs = scanner.scan_roots(&[drive.clone(), dir.path().join("USB")]);
        assert_eq!(txs.len(), 1);
        let tx = &txs[0];
        assert_eq!(tx.id, "TX_MIC002_20261117_121733");
        assert_eq!(tx.slot, Some(1));
        assert_eq!(tx.model.as_deref(), Some("MIC 3"));
        assert_eq!(tx.firmware.as_deref(), Some("03.01.00.10"));
        assert_eq!(tx.takes.len(), 2);
        let newest = &tx.takes[0];
        assert_eq!(newest.key, "TX01_MIC003_20261123_232008");
        assert_eq!(newest.day(), "2026-11-23");
        assert_eq!(newest.files.len(), 2);
        assert_eq!(newest.files[0].kind, FileKind::Original);
        assert_eq!(newest.files[1].kind, FileKind::Processed);
        assert!((newest.info.as_ref().unwrap().duration_secs - 1.0).abs() < 1e-9);
        assert_eq!(tx.takes[1].files.len(), 1);
    }
}
