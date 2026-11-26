//! What a device link has done since it opened, kept for the diagnostic
//! report (see [`DeviceManager::diagnostics`](crate::DeviceManager::diagnostics)).
//!
//! Every actor records into a [`Trace`]: the first and the latest raw reads
//! and frames, how many frames of each command it saw, the commands it sent
//! and how the device answered, and model switches. It's bounded, so it can
//! stay on for the whole session.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;

use crate::actor::now_millis;

/// Raw reads kept from the start of the session (for devices whose bytes
/// never parse as frames).
const FIRST_READS: usize = 4;
/// Frames kept from the start of the session.
const FIRST_FRAMES: usize = 12;
/// Latest frames kept.
const RECENT_FRAMES: usize = 12;
/// Latest commands kept.
const RECENT_COMMANDS: usize = 24;

/// One command written to the device and what came back.
struct CommandLog {
    at_ms: u64,
    what: String,
    packet: Vec<u8>,
    outcome: Option<String>,
}

/// The bounded record of one device link.
pub struct Trace {
    opened_ms: u64,
    reads: u64,
    read_bytes: u64,
    frames: u64,
    first_reads: Vec<Vec<u8>>,
    first_frames: Vec<Vec<u8>>,
    recent_frames: VecDeque<Vec<u8>>,
    /// `(cmd_set, cmd_id)` → frames seen.
    kinds: BTreeMap<(u8, u8), u64>,
    commands: VecDeque<CommandLog>,
    events: Vec<String>,
}

impl Default for Trace {
    fn default() -> Self {
        Self::new()
    }
}

impl Trace {
    pub fn new() -> Self {
        Self {
            opened_ms: now_millis(),
            reads: 0,
            read_bytes: 0,
            frames: 0,
            first_reads: Vec::new(),
            first_frames: Vec::new(),
            recent_frames: VecDeque::new(),
            kinds: BTreeMap::new(),
            commands: VecDeque::new(),
            events: Vec::new(),
        }
    }

    fn elapsed(&self) -> String {
        secs(now_millis().saturating_sub(self.opened_ms))
    }

    /// A chunk of bytes as read from the link.
    pub fn read(&mut self, data: &[u8]) {
        self.reads += 1;
        self.read_bytes += data.len() as u64;
        if self.first_reads.len() < FIRST_READS {
            self.first_reads.push(data.to_vec());
        }
    }

    /// A complete frame taken from the reads.
    pub fn frame(&mut self, frame: &[u8]) {
        self.frames += 1;
        if let (Some(&set), Some(&id)) = (frame.get(9), frame.get(10)) {
            *self.kinds.entry((set, id)).or_default() += 1;
        }
        if self.first_frames.len() < FIRST_FRAMES {
            self.first_frames.push(frame.to_vec());
        } else {
            if self.recent_frames.len() == RECENT_FRAMES {
                self.recent_frames.pop_front();
            }
            self.recent_frames.push_back(frame.to_vec());
        }
    }

    /// A command was written; its outcome follows with [`outcome`](Self::outcome).
    pub fn command(&mut self, what: String, packet: &[u8]) {
        if self.commands.len() == RECENT_COMMANDS {
            self.commands.pop_front();
        }
        self.commands.push_back(CommandLog {
            at_ms: now_millis(),
            what,
            packet: packet.to_vec(),
            outcome: None,
        });
    }

    /// How the device answered the last command.
    pub fn outcome(&mut self, outcome: impl Into<String>) {
        if let Some(c) = self.commands.back_mut() {
            if c.outcome.is_none() {
                c.outcome = Some(outcome.into());
            }
        }
    }

    /// Something worth noting (a model switch, the link closing).
    pub fn event(&mut self, what: impl std::fmt::Display) {
        if self.events.len() < 32 {
            let line = format!("+{} {what}", self.elapsed());
            self.events.push(line);
        }
    }

    /// The trace as report text. Occurrences of `hide` (serial numbers) in
    /// raw bytes are masked.
    pub fn render(&self, out: &mut String, hide: &[String]) {
        let _ = writeln!(
            out,
            "  link open for {} · {} reads ({} bytes) · {} frames",
            self.elapsed(),
            self.reads,
            self.read_bytes,
            self.frames
        );
        if !self.kinds.is_empty() {
            let kinds: Vec<String> = self
                .kinds
                .iter()
                .map(|((s, i), n)| format!("{s:02x}/{i:02x}×{n}"))
                .collect();
            let _ = writeln!(out, "  frames by cmd set/id: {}", kinds.join(" "));
        }
        if !self.events.is_empty() {
            let _ = writeln!(out, "  events:");
            for e in &self.events {
                let _ = writeln!(out, "    {e}");
            }
        }
        if !self.commands.is_empty() {
            let _ = writeln!(out, "  commands sent (latest {RECENT_COMMANDS}):");
            for c in &self.commands {
                let _ = writeln!(
                    out,
                    "    +{} {} → {}\n      {}",
                    secs(c.at_ms.saturating_sub(self.opened_ms)),
                    c.what,
                    c.outcome.as_deref().unwrap_or("waiting"),
                    hex(&c.packet, hide)
                );
            }
        }
        if self.frames == 0 && !self.first_reads.is_empty() {
            let _ = writeln!(out, "  first raw reads (no frame parsed):");
            for r in &self.first_reads {
                let _ = writeln!(out, "    {}", hex(r, hide));
            }
        }
        if !self.first_frames.is_empty() {
            let _ = writeln!(out, "  first frames:");
            for f in &self.first_frames {
                let _ = writeln!(out, "    {}", hex(f, hide));
            }
        }
        if !self.recent_frames.is_empty() {
            let _ = writeln!(out, "  latest frames:");
            for f in &self.recent_frames {
                let _ = writeln!(out, "    {}", hex(f, hide));
            }
        }
    }
}

/// Milliseconds as `12.3s`.
pub fn secs(ms: u64) -> String {
    format!("{}.{}s", ms / 1000, (ms % 1000) / 100)
}

/// Bytes as hex, with every occurrence of a string in `hide` replaced by
/// `**` bytes so serial numbers don't end up in a shared report.
pub fn hex(data: &[u8], hide: &[String]) -> String {
    let mut masked = vec![false; data.len()];
    for s in hide.iter().map(String::as_bytes).filter(|s| s.len() >= 4) {
        let mut i = 0;
        while i + s.len() <= data.len() {
            if &data[i..i + s.len()] == s {
                masked[i..i + s.len()].iter_mut().for_each(|m| *m = true);
                i += s.len();
            } else {
                i += 1;
            }
        }
    }
    let mut out = String::with_capacity(data.len() * 3);
    for (b, m) in data.iter().zip(masked) {
        if !out.is_empty() {
            out.push(' ');
        }
        if m {
            out.push_str("**");
        } else {
            let _ = write!(out, "{b:02x}");
        }
    }
    out
}

/// A serial number cut down to its last four characters.
pub fn redact(serial: &str) -> String {
    let tail: String = serial
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("…{tail}")
}

/// `YYYY-MM-DD HH:MM UTC` for a Unix time in milliseconds.
pub fn utc(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        (rem % 3600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_masks_serials() {
        let data = b"\x55\x10ABCD1234\x00";
        assert_eq!(
            hex(data, &["ABCD1234".into()]),
            "55 10 ** ** ** ** ** ** ** ** 00"
        );
        assert_eq!(hex(&[0x55, 0x0a], &[]), "55 0a");
    }

    #[test]
    fn utc_formats_dates() {
        assert_eq!(utc(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc(1_795_699_260_000), "2026-11-26 13:21 UTC");
    }

    #[test]
    fn redact_keeps_last_four() {
        assert_eq!(redact("4BBCL9X00A1234"), "…1234");
        assert_eq!(redact("ab"), "…ab");
    }

    #[test]
    fn render_lists_commands_and_masks_serials() {
        let mut t = Trace::new();
        t.read(&[0x55, 0x01]);
        t.frame(b"\x55\x10\x04\x00\x00\x00\x00\x00\x00\x2b\x06SER123456");
        t.command("set noise-cancel=strong".into(), &[0x55, 0x0e]);
        t.outcome("rejected (0xff)");
        t.event("model a -> b");
        let mut out = String::new();
        t.render(&mut out, &["SER123456".into()]);
        assert!(out.contains("1 reads (2 bytes) · 1 frames"), "{out}");
        assert!(out.contains("frames by cmd set/id: 2b/06×1"), "{out}");
        assert!(
            out.contains("set noise-cancel=strong → rejected (0xff)"),
            "{out}"
        );
        assert!(out.contains("model a -> b"), "{out}");
        assert!(out.contains("2b 06 ** ** ** ** ** ** ** ** **"), "{out}");
        assert!(
            !out.contains("first raw reads"),
            "frames parsed, raw reads not needed: {out}"
        );
    }

    #[test]
    fn trace_keeps_first_and_latest_frames() {
        let mut t = Trace::new();
        for i in 0..40u8 {
            t.frame(&[0x55, 0, 0, 0, 0, 0, 0, 0, 0, 0x03, i]);
        }
        assert_eq!(t.first_frames.len(), FIRST_FRAMES);
        assert_eq!(t.recent_frames.len(), RECENT_FRAMES);
        assert_eq!(t.recent_frames.back().unwrap()[10], 39);
        assert_eq!(t.kinds.len(), 40);
    }
}
