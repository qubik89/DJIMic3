//! Copying takes off the transmitters, and removing the ones safely copied.
//!
//! Files land in `<dest>/<YYYY-MM-DD>/<original name>`. Each is written to a
//! `.part` file, synced, read back and compared with what was read from the
//! transmitter before it gets its real name, so a file in the import folder
//! is always complete. A file already there with the same size is skipped.

use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{DefaultHasher, Hasher};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::scan::{RecordingFile, Take, Transmitter};

const CHUNK: usize = 1 << 20;

/// Where `file` of `take` is imported to under `dest`.
pub fn dest_path(dest: &Path, take: &Take, file: &RecordingFile) -> PathBuf {
    dest.join(take.day()).join(&file.name)
}

/// Set `imported` on every file already in `dest` with the same size.
pub fn mark_imported(txs: &mut [Transmitter], dest: &Path) {
    for tx in txs {
        for take in &mut tx.takes {
            let day = take.day().to_string();
            for f in &mut take.files {
                f.imported = fs::metadata(dest.join(&day).join(&f.name)).map(|m| m.len() == f.size).unwrap_or(false);
            }
        }
    }
}

/// One file to copy.
#[derive(Debug, Clone)]
pub struct Job {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub size: u64,
}

/// Index every listed file by its path.
fn index(txs: &[Transmitter]) -> HashMap<&Path, (&Take, &RecordingFile)> {
    txs.iter()
        .flat_map(|tx| tx.takes.iter())
        .flat_map(|take| take.files.iter().map(move |f| (f.path.as_path(), (take, f))))
        .collect()
}

/// Turn the requested source paths into copy jobs. Only files found on a
/// transmitter by the last scan are accepted.
pub fn plan(txs: &[Transmitter], files: &[PathBuf], dest: &Path) -> Result<Vec<Job>, String> {
    let known = index(txs);
    files
        .iter()
        .map(|p| {
            let (take, f) = known
                .get(p.as_path())
                .ok_or_else(|| format!("{} is not on a connected transmitter", p.display()))?;
            Ok(Job { src: f.path.clone(), dst: dest_path(dest, take, f), size: f.size })
        })
        .collect()
}

/// Copy progress, sent a few times a second.
#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub done_bytes: u64,
    pub total_bytes: u64,
    /// 0-based index of the file being copied.
    pub index: usize,
    pub count: usize,
    pub file: String,
}

/// A file that couldn't be copied or deleted.
#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub file: String,
    pub error: String,
}

/// How an import went.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Outcome {
    pub copied: usize,
    /// Already in the import folder.
    pub skipped: usize,
    pub failed: Vec<Failure>,
    pub cancelled: bool,
    pub bytes: u64,
}

enum Copied {
    New,
    AlreadyThere,
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Run `jobs` in order. `cancel` is checked between chunks; `progress` gets
/// called as data moves.
pub fn run(jobs: &[Job], cancel: &AtomicBool, mut progress: impl FnMut(&Progress)) -> Outcome {
    let total_bytes: u64 = jobs.iter().map(|j| j.size).sum();
    let mut out = Outcome::default();
    let mut base = 0u64;
    for (index, job) in jobs.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            out.cancelled = true;
            break;
        }
        let file = file_name(&job.src);
        let mut report = |done: u64| {
            progress(&Progress { done_bytes: base + done, total_bytes, index, count: jobs.len(), file: file.clone() })
        };
        report(0);
        match copy_verified(&job.src, &job.dst, cancel, &mut report) {
            Ok(Copied::New) => {
                out.copied += 1;
                out.bytes += job.size;
            }
            Ok(Copied::AlreadyThere) => out.skipped += 1,
            Err(e) if e.kind() == io::ErrorKind::Interrupted && cancel.load(Ordering::Relaxed) => {
                out.cancelled = true;
                break;
            }
            Err(e) => out.failed.push(Failure { file: file.clone(), error: e.to_string() }),
        }
        base += job.size;
    }
    if !out.cancelled {
        progress(&Progress {
            done_bytes: total_bytes,
            total_bytes,
            index: jobs.len().saturating_sub(1),
            count: jobs.len(),
            file: String::new(),
        });
    }
    out
}

/// `name.wav` → `name (2).wav`, `name (3).wav`, … until one is free.
fn free_name(dst: &Path) -> PathBuf {
    let stem = dst.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = dst.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
    (2..)
        .map(|n| dst.with_file_name(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("unbounded range")
}

fn copy_verified(src: &Path, dst: &Path, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> io::Result<Copied> {
    let src_meta = fs::metadata(src)?;
    let size = src_meta.len();
    let mut dst = dst.to_path_buf();
    if let Ok(m) = fs::metadata(&dst) {
        if m.len() == size {
            return Ok(Copied::AlreadyThere);
        }
        // A different file with the same name: keep both.
        dst = free_name(&dst);
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = dst.with_file_name(format!("{}.part", file_name(&dst)));

    let result = (|| {
        let mut input = File::open(src)?;
        let mut output = File::create(&part)?;
        let mut hash = DefaultHasher::new();
        let mut buf = vec![0u8; CHUNK];
        let mut done = 0u64;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hash.write(&buf[..n]);
            output.write_all(&buf[..n])?;
            done += n as u64;
            progress(done);
        }
        if done != size {
            return Err(io::Error::other(format!("read {done} of {size} bytes")));
        }
        if let Ok(modified) = src_meta.modified() {
            let _ = output.set_modified(modified);
        }
        output.sync_all()?;
        drop(output);

        // Read the copy back and compare it with what came off the transmitter.
        let mut check = DefaultHasher::new();
        let mut copy = File::open(&part)?;
        let mut len = 0u64;
        loop {
            let n = copy.read(&mut buf)?;
            if n == 0 {
                break;
            }
            check.write(&buf[..n]);
            len += n as u64;
        }
        if len != size || check.finish() != hash.finish() {
            return Err(io::Error::other("the copy doesn't match the original"));
        }
        fs::rename(&part, &dst)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&part);
    }
    result.map(|_| Copied::New)
}

/// How a delete went.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Deleted {
    pub deleted: usize,
    pub failed: Vec<Failure>,
}

/// Delete the requested files from the transmitters, but only those found by
/// the last scan that are already in `dest` with the same size.
pub fn delete_imported(txs: &[Transmitter], files: &[PathBuf], dest: &Path) -> Deleted {
    let known = index(txs);
    let mut out = Deleted::default();
    for p in files {
        let name = file_name(p);
        let Some((take, f)) = known.get(p.as_path()) else {
            out.failed.push(Failure { file: name, error: "not on a connected transmitter".into() });
            continue;
        };
        let copy = dest_path(dest, take, f);
        match fs::metadata(&copy) {
            Ok(m) if m.len() == f.size => match fs::remove_file(&f.path) {
                Ok(()) => out.deleted += 1,
                Err(e) => out.failed.push(Failure { file: name, error: e.to_string() }),
            },
            _ => out.failed.push(Failure { file: name, error: "not imported yet".into() }),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::Scanner;
    use crate::testdir::TestDir;
    use crate::wav::tests::mic3_wav;

    fn drive(dir: &TestDir) -> PathBuf {
        let drive = dir.path().join("NO NAME");
        let folder = drive.join("TX_MIC001_20261117_114545");
        fs::create_dir_all(&folder).unwrap();
        let wav = mic3_wav(1, 24, 48_000 * 3, "2026-11-23", "23:20:07");
        fs::write(folder.join("TX02_MIC006_20261123_232007_orig.wav"), &wav).unwrap();
        fs::write(folder.join("TX02_MIC006_20261123_232007_edit.wav"), &wav).unwrap();
        fs::write(folder.join("TX02_MIC001_20261117_114545_orig.wav"), mic3_wav(1, 24, 100, "2026-11-17", "11:45:45")).unwrap();
        drive
    }

    #[test]
    fn imports_skips_and_deletes() {
        let dir = TestDir::new("import");
        let drive = drive(&dir);
        let dest = dir.path().join("Music").join("DJI Mic");
        let scanner = Scanner::new();

        let txs = scanner.scan_roots(std::slice::from_ref(&drive));
        let all: Vec<PathBuf> = txs[0].takes.iter().flat_map(|t| t.files.iter().map(|f| f.path.clone())).collect();
        let jobs = plan(&txs, &all, &dest).unwrap();
        let mut last = None;
        let out = run(&jobs, &AtomicBool::new(false), |p| last = Some(p.clone()));
        assert_eq!((out.copied, out.skipped, out.failed.len(), out.cancelled), (3, 0, 0, false));
        let last = last.unwrap();
        assert_eq!(last.done_bytes, last.total_bytes);

        let copy = dest.join("2026-11-23").join("TX02_MIC006_20261123_232007_edit.wav");
        assert_eq!(fs::read(&copy).unwrap(), fs::read(jobs[0].src.with_file_name("TX02_MIC006_20261123_232007_edit.wav")).unwrap());
        assert!(dest.join("2026-11-17").join("TX02_MIC001_20261117_114545_orig.wav").exists());
        assert!(fs::read_dir(dest.join("2026-11-23")).unwrap().all(|e| !e.unwrap().file_name().to_string_lossy().ends_with(".part")));

        let mut txs = scanner.scan_roots(std::slice::from_ref(&drive));
        mark_imported(&mut txs, &dest);
        assert!(txs[0].takes.iter().flat_map(|t| &t.files).all(|f| f.imported));

        // Second run copies nothing.
        let out = run(&plan(&txs, &all, &dest).unwrap(), &AtomicBool::new(false), |_| {});
        assert_eq!((out.copied, out.skipped), (0, 3));

        // Deleting only touches files that were imported.
        fs::remove_file(&copy).unwrap();
        let gone = delete_imported(&txs, &all, &dest);
        assert_eq!(gone.deleted, 2);
        assert_eq!(gone.failed.len(), 1);
        assert!(drive.join("TX_MIC001_20261117_114545").join("TX02_MIC006_20261123_232007_edit.wav").exists());
        assert!(!drive.join("TX_MIC001_20261117_114545").join("TX02_MIC006_20261123_232007_orig.wav").exists());
    }

    #[test]
    fn rejects_files_not_on_a_transmitter() {
        let dir = TestDir::new("reject");
        let drive = drive(&dir);
        let txs = Scanner::new().scan_roots(&[drive]);
        let elsewhere = dir.path().join("secret.wav");
        fs::write(&elsewhere, b"x").unwrap();
        assert!(plan(&txs, std::slice::from_ref(&elsewhere), dir.path()).is_err());
        let out = delete_imported(&txs, std::slice::from_ref(&elsewhere), dir.path());
        assert_eq!(out.deleted, 0);
        assert!(elsewhere.exists());
    }

    #[test]
    fn keeps_a_different_file_with_the_same_name() {
        let dir = TestDir::new("clash");
        let drive = drive(&dir);
        let dest = dir.path().join("out");
        let txs = Scanner::new().scan_roots(&[drive]);
        let take = &txs[0].takes[0];
        let f = &take.files[0];
        let existing = dest_path(&dest, take, f);
        fs::create_dir_all(existing.parent().unwrap()).unwrap();
        fs::write(&existing, b"something else").unwrap();
        let out = run(&plan(&txs, std::slice::from_ref(&f.path), &dest).unwrap(), &AtomicBool::new(false), |_| {});
        assert_eq!(out.copied, 1);
        assert_eq!(fs::read(&existing).unwrap(), b"something else");
        let stem = existing.file_stem().unwrap().to_string_lossy().into_owned();
        assert!(existing.with_file_name(format!("{stem} (2).wav")).exists());
    }

    #[test]
    fn cancelling_leaves_no_partial_file() {
        let dir = TestDir::new("cancel");
        let drive = drive(&dir);
        let dest = dir.path().join("out");
        let txs = Scanner::new().scan_roots(&[drive]);
        let all: Vec<PathBuf> = txs[0].takes.iter().flat_map(|t| t.files.iter().map(|f| f.path.clone())).collect();
        let cancel = AtomicBool::new(false);
        let out = run(&plan(&txs, &all, &dest).unwrap(), &cancel, |p| {
            if p.done_bytes > 0 {
                cancel.store(true, Ordering::Relaxed);
            }
        });
        assert!(out.cancelled);
        let leftovers: Vec<_> = fs::read_dir(dest.join("2026-11-23")).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
}
