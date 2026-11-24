//! Downloading recordings without the charging case, the way DJI Mimo does.
//!
//! 1. Pair over Bluetooth and join the transmitter's own Wi-Fi (the `link`
//!    crate; the transmitter buzzes twice and the user presses its link
//!    button once).
//! 2. Open the DUML control session on TCP 60000, which also starts an HTTP
//!    file server ([`session`]).
//! 3. `files_info.json` lists the takes; each WAV is a plain HTTP download.
//!
//! The protocol is documented in PROTOCOL.md ("Wireless file download").

mod session;

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use link::{Joined, Unit};

use crate::import::{Failure, Job, Outcome, Progress};
use crate::scan::{build_transmitter, Entry, Transmitter};
use crate::wav;

pub use link::{new_app_id, Error, Step};
pub use session::RemoteFile;

/// A transmitter reached over its Wi-Fi.
pub struct Connection {
    session: session::Session,
    joined: Joined,
    transmitter: Transmitter,
}

/// Pair with a transmitter, join its Wi-Fi, open the session and list the
/// takes, retrying the Bluetooth part until the user confirms on the
/// transmitter or `cancel` is set.
pub async fn connect(app_id: &str, country: &str, known: &[link::Known], cancel: &AtomicBool, step: &(dyn Fn(Step) + Send + Sync)) -> Result<Connection, Error> {
    let joined = link::pair_and_join(Unit::Transmitter, app_id, country, known, cancel, step).await?;
    let result = tokio::task::spawn_blocking(move || {
        let opened = session::Session::open(joined.iface().clone(), Duration::from_secs(20)).and_then(|session| {
            let transmitter = describe(&session, &joined.ssid)?;
            Ok((session, transmitter))
        });
        match opened {
            Ok((session, transmitter)) => Ok(Connection { session, joined, transmitter }),
            Err(e) => {
                joined.leave();
                Err(e)
            }
        }
    })
    .await
    .map_err(|e| Error::Session(e.to_string()))?;
    if result.is_ok() {
        step(Step::Ready);
    }
    result
}

/// List the takes, reading each file's WAV header with a ranged request.
fn describe(session: &session::Session, ssid: &str) -> Result<Transmitter, Error> {
    let files = session.list()?;
    let entries = files
        .into_iter()
        .filter(|f| f.fname.to_ascii_lowercase().ends_with(".wav"))
        .map(|f| {
            let name = f.fname.rsplit('/').next().unwrap_or(&f.fname).to_string();
            let info = session.head(&f.fname, 64 * 1024).ok().and_then(|h| wav::parse(&mut Cursor::new(&h), f.fsize).ok());
            Entry { name, path: PathBuf::from(&f.fname), size: f.fsize, info }
        })
        .collect();
    let mut tx = build_transmitter(ssid.to_string(), PathBuf::from(format!("wifi://{ssid}")), entries);
    tx.wireless = true;
    Ok(tx)
}

impl Connection {
    /// The access point, to remember for next time.
    pub fn known(&self) -> link::Known {
        link::Known { ssid: self.joined.ssid.clone(), psk: self.joined.psk.clone() }
    }

    pub fn transmitter(&self) -> &Transmitter {
        &self.transmitter
    }

    pub fn is_alive(&self) -> bool {
        self.session.is_alive()
    }

    /// Download `jobs` (sources are server paths), same contract as
    /// [`crate::run`]: files land complete or not at all.
    pub fn download(&self, jobs: &[Job], cancel: &AtomicBool, mut progress: impl FnMut(&Progress)) -> Outcome {
        let total_bytes: u64 = jobs.iter().map(|j| j.size).sum();
        let mut out = Outcome::default();
        let mut base = 0u64;
        for (index, job) in jobs.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                out.cancelled = true;
                break;
            }
            let file = job.dst.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut report = |done: u64| {
                progress(&Progress { done_bytes: base + done, total_bytes, index, count: jobs.len(), file: file.clone() })
            };
            report(0);
            match self.fetch(job, cancel, &mut report) {
                Ok(true) => {
                    out.copied += 1;
                    out.bytes += job.size;
                }
                Ok(false) => out.skipped += 1,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted && cancel.load(Ordering::Relaxed) => {
                    out.cancelled = true;
                    break;
                }
                Err(e) => out.failed.push(Failure { file: file.clone(), error: e.to_string() }),
            }
            base += job.size;
        }
        if !out.cancelled {
            progress(&Progress { done_bytes: total_bytes, total_bytes, index: jobs.len().saturating_sub(1), count: jobs.len(), file: String::new() });
        }
        out
    }

    /// `Ok(false)` when the file is already there.
    fn fetch(&self, job: &Job, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> std::io::Result<bool> {
        if fs::metadata(&job.dst).map(|m| m.len() == job.size).unwrap_or(false) {
            return Ok(false);
        }
        if let Some(parent) = job.dst.parent() {
            fs::create_dir_all(parent)?;
        }
        let part = part_path(&job.dst);
        let fname = job.src.to_string_lossy().replace('\\', "/");
        // One retry resumes an interrupted transfer where it stopped.
        let mut result = self.session.download(&fname, job.size, &part, cancel, progress);
        if matches!(&result, Err(e) if e.kind() != std::io::ErrorKind::Interrupted) && self.session.is_alive() {
            result = self.session.download(&fname, job.size, &part, cancel, progress);
        }
        result?;
        // A complete WAV with the expected length.
        if fs::metadata(&part)?.len() != job.size || wav::read(&part).is_err() {
            let _ = fs::remove_file(&part);
            return Err(std::io::Error::other("the downloaded file is incomplete"));
        }
        fs::rename(&part, &job.dst)?;
        Ok(true)
    }

    /// Close the session and give the Wi-Fi back to the usual network.
    pub fn close(self) {
        let Connection { session, joined, .. } = self;
        drop(session);
        joined.leave();
    }
}

fn part_path(dst: &Path) -> PathBuf {
    let name = dst.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    dst.with_file_name(format!("{name}.part"))
}
