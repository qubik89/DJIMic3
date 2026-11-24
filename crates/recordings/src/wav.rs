//! Just enough RIFF/WAVE parsing to describe a recording: format, length and
//! the Broadcast Wave (`bext`) fields the transmitters fill in.
//!
//! Only chunk headers and the small `fmt `/`bext`/`ds64` bodies are read; the
//! audio itself is never touched, so this is cheap even over USB.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use serde::Serialize;

/// What a WAV header says about the audio in it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WavInfo {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    /// IEEE float samples (32-bit float recording).
    pub float: bool,
    /// Length of the audio in seconds.
    pub duration_secs: f64,
    /// `bext` originator, e.g. `MIC 3`.
    pub originator: Option<String>,
    /// `bext` description, e.g. `ver:03.01.00.10` (the transmitter firmware).
    pub description: Option<String>,
    /// `bext` origination date and time, `YYYY-MM-DD` and `hh:mm:ss`.
    pub origination: Option<(String, String)>,
}

const WAVE_FORMAT_PCM: u16 = 0x0001;
const WAVE_FORMAT_IEEE_FLOAT: u16 = 0x0003;
const WAVE_FORMAT_EXTENSIBLE: u16 = 0xfffe;

/// Read the header of the WAV file at `path`.
pub fn read(path: &Path) -> io::Result<WavInfo> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    parse(&mut file, len)
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn u16le(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

fn u64le(b: &[u8]) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[..8]);
    u64::from_le_bytes(a)
}

/// NUL-terminated, Latin-1 text field; `None` when empty.
fn text(b: &[u8]) -> Option<String> {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    let s: String = b[..end].iter().map(|&c| c as char).collect();
    let s = s.trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Parse a WAV stream of `len` bytes.
pub fn parse<R: Read + Seek>(r: &mut R, len: u64) -> io::Result<WavInfo> {
    let mut head = [0u8; 12];
    r.read_exact(&mut head)?;
    let rf64 = match &head[0..4] {
        b"RIFF" => false,
        b"RF64" => true,
        _ => return Err(invalid("not a RIFF file")),
    };
    if &head[8..12] != b"WAVE" {
        return Err(invalid("not a WAVE file"));
    }

    let mut fmt: Option<(u16, u16, u32, u32, u16)> = None; // tag, channels, rate, byte rate, bits
    let mut bext: Option<Vec<u8>> = None;
    let mut ds64_data: Option<u64> = None;
    let mut data: Option<(u64, u64)> = None; // offset, declared size

    let mut pos = 12u64;
    while pos + 8 <= len {
        r.seek(SeekFrom::Start(pos))?;
        let mut ch = [0u8; 8];
        r.read_exact(&mut ch)?;
        let id = [ch[0], ch[1], ch[2], ch[3]];
        let size = u32le(&ch[4..8]) as u64;
        let body = pos + 8;
        match &id {
            b"fmt " => {
                if size < 16 {
                    return Err(invalid("short fmt chunk"));
                }
                let mut b = vec![0u8; size.min(40) as usize];
                r.read_exact(&mut b)?;
                let mut tag = u16le(&b[0..2]);
                if tag == WAVE_FORMAT_EXTENSIBLE && size >= 40 {
                    // The sub-format GUID starts with the real format tag.
                    tag = u16le(&b[24..26]);
                }
                fmt = Some((tag, u16le(&b[2..4]), u32le(&b[4..8]), u32le(&b[8..12]), u16le(&b[14..16])));
            }
            b"bext" => {
                let mut b = vec![0u8; size.min(602) as usize];
                r.read_exact(&mut b)?;
                bext = Some(b);
            }
            b"ds64" if rf64 => {
                let mut b = [0u8; 24];
                r.read_exact(&mut b)?;
                ds64_data = Some(u64le(&b[8..16]));
            }
            b"data" => {
                data = Some((body, size));
                break;
            }
            _ => {}
        }
        pos = body + size + (size & 1);
    }

    let (tag, channels, sample_rate, byte_rate, bits_per_sample) = fmt.ok_or_else(|| invalid("no fmt chunk"))?;
    if tag != WAVE_FORMAT_PCM && tag != WAVE_FORMAT_IEEE_FLOAT {
        return Err(invalid("unsupported sample format"));
    }
    let (offset, declared) = data.ok_or_else(|| invalid("no data chunk"))?;
    let available = len.saturating_sub(offset);
    let mut size = if rf64 && declared == 0xffff_ffff { ds64_data.unwrap_or(available) } else { declared };
    // A recording cut short (battery out) can leave the size unset or too big.
    if size == 0 || size > available {
        size = available;
    }
    let duration_secs = if byte_rate > 0 { size as f64 / byte_rate as f64 } else { 0.0 };

    let (originator, description, origination) = match &bext {
        Some(b) if b.len() >= 338 => {
            let date = text(&b[320..330]);
            let time = text(&b[330..338]);
            (text(&b[256..288]), text(&b[0..256]), date.zip(time))
        }
        _ => (None, None, None),
    };

    Ok(WavInfo {
        channels,
        sample_rate,
        bits_per_sample,
        float: tag == WAVE_FORMAT_IEEE_FLOAT,
        duration_secs,
        originator,
        description,
        origination,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Cursor;

    /// A WAV laid out like the Mic 3's: fmt, bext, iXML, PAD, data at 32776.
    pub(crate) fn mic3_wav(tag: u16, bits: u16, samples: u32, date: &str, time: &str) -> Vec<u8> {
        let rate = 48_000u32;
        let block = (bits / 8) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF\0\0\0\0WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&tag.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * block).to_le_bytes());
        v.extend_from_slice(&(block as u16).to_le_bytes());
        v.extend_from_slice(&bits.to_le_bytes());
        let mut bext = vec![0u8; 602];
        bext[..15].copy_from_slice(b"ver:03.01.00.10");
        bext[256..261].copy_from_slice(b"MIC 3");
        bext[320..330].copy_from_slice(date.as_bytes());
        bext[330..338].copy_from_slice(time.as_bytes());
        v.extend_from_slice(b"bext");
        v.extend_from_slice(&602u32.to_le_bytes());
        v.extend_from_slice(&bext);
        let pad = 32776 - v.len() - 8 - 8;
        v.extend_from_slice(b"PAD ");
        v.extend_from_slice(&(pad as u32).to_le_bytes());
        v.resize(v.len() + pad, 0);
        let size = samples * block;
        v.extend_from_slice(b"data");
        v.extend_from_slice(&size.to_le_bytes());
        assert_eq!(v.len(), 32776);
        v.resize(v.len() + size as usize, 0x11);
        let riff = (v.len() - 8) as u32;
        v[4..8].copy_from_slice(&riff.to_le_bytes());
        v
    }

    #[test]
    fn reads_mic3_header() {
        let wav = mic3_wav(1, 24, 48_000 * 2, "2026-11-23", "23:20:08");
        let info = parse(&mut Cursor::new(&wav), wav.len() as u64).unwrap();
        assert_eq!(info.channels, 1);
        assert_eq!(info.sample_rate, 48_000);
        assert_eq!(info.bits_per_sample, 24);
        assert!(!info.float);
        assert!((info.duration_secs - 2.0).abs() < 1e-9);
        assert_eq!(info.originator.as_deref(), Some("MIC 3"));
        assert_eq!(info.description.as_deref(), Some("ver:03.01.00.10"));
        assert_eq!(info.origination, Some(("2026-11-23".into(), "23:20:08".into())));
    }

    #[test]
    fn reads_float_and_truncated_data() {
        let mut wav = mic3_wav(3, 32, 48_000, "2026-11-23", "23:20:08");
        // Data size never written (recording interrupted).
        wav[32772..32776].copy_from_slice(&0u32.to_le_bytes());
        let info = parse(&mut Cursor::new(&wav), wav.len() as u64).unwrap();
        assert!(info.float);
        assert!((info.duration_secs - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_wav() {
        let junk = b"ID3\x04 not a wave file".to_vec();
        assert!(parse(&mut Cursor::new(&junk), junk.len() as u64).is_err());
    }
}
