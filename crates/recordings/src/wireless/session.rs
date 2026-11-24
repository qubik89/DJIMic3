//! Talking to the transmitter on its access point: the DUML control session
//! on TCP 60000 (kept alive like DJI Mimo does) and the HTTP file server it
//! starts on port 80.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use link::duml::{self, Reassembler, APP, BROADCAST, REQUEST, TX};
use link::{Error, Iface};

use link::CONTROL_PORT;
const HTTP_PORT: u16 = 80;
const FILES_INFO: &str = "data/emmc0/files_info.json";

/// One entry of `files_info.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteFile {
    /// Server path, e.g. `data/emmc0/TX_MIC002_…/TX01_MIC003_…_orig.wav`.
    pub fname: String,
    pub fsize: u64,
    #[serde(default)]
    pub duration_sec: f64,
}

#[derive(Deserialize)]
struct FilesInfo {
    files: Vec<RemoteFile>,
}

/// An open session with the transmitter.
pub struct Session {
    iface: Iface,
    control: TcpStream,
    alive: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl Session {
    /// Connect the control channel (retrying while DHCP settles), keep it
    /// alive and start the file server.
    pub fn open(iface: Iface, timeout: Duration) -> Result<Self, Error> {
        let deadline = Instant::now() + timeout;
        let control = loop {
            match iface.connect(CONTROL_PORT, Duration::from_secs(2)) {
                Ok(s) => break s,
                Err(e) if Instant::now() >= deadline => return Err(Error::Session(format!("control channel: {e}"))),
                Err(_) => std::thread::sleep(Duration::from_millis(500)),
            }
        };
        let alive = Arc::new(AtomicBool::new(true));
        let stop = Arc::new(AtomicBool::new(false));
        let seq = Arc::new(Mutex::new(0x4000u16));
        let file_server = Arc::new(AtomicBool::new(false));

        // Reader: drain frames, note the file server's answer, notice EOF.
        {
            let mut rx = control.try_clone().map_err(|e| Error::Session(e.to_string()))?;
            let alive = alive.clone();
            let file_server = file_server.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                let mut frames = Reassembler::default();
                loop {
                    match rx.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            for f in frames.push(&buf[..n]) {
                                if f.src == TX && f.is(0x5b, 0x19) && f.is_response() {
                                    file_server.store(f.payload.first() == Some(&0), Ordering::SeqCst);
                                }
                            }
                        }
                    }
                }
                alive.store(false, Ordering::SeqCst);
            });
        }

        // Writer: ping every 0.5 s and the app heartbeat every 2.5 s, as Mimo.
        {
            let mut tx = control.try_clone().map_err(|e| Error::Session(e.to_string()))?;
            let (stop, alive, seq) = (stop.clone(), alive.clone(), seq.clone());
            std::thread::spawn(move || {
                let mut n = 0u32;
                while !stop.load(Ordering::SeqCst) && alive.load(Ordering::SeqCst) {
                    let mut frames = Vec::new();
                    {
                        let mut s = seq.lock().unwrap_or_else(|e| e.into_inner());
                        frames.push(duml::encode(*s, APP, TX, REQUEST, 0x00, 0x00, &[0x01, 0x00]));
                        *s = s.wrapping_add(1);
                        if n.is_multiple_of(5) {
                            frames.push(duml::encode(*s, APP, BROADCAST, REQUEST, 0x00, 0x2b, &[0x04, 0x00]));
                            *s = s.wrapping_add(1);
                        }
                    }
                    if frames.iter().any(|f| tx.write_all(f).is_err()) {
                        alive.store(false, Ordering::SeqCst);
                        break;
                    }
                    n += 1;
                    std::thread::sleep(Duration::from_millis(500));
                }
            });
        }

        // Start the file server: 5b/19 01 01 00 01, answered with 00.
        let start = {
            let mut s = seq.lock().unwrap_or_else(|e| e.into_inner());
            let f = duml::encode(*s, APP, TX, REQUEST, 0x5b, 0x19, &[0x01, 0x01, 0x00, 0x01]);
            *s = s.wrapping_add(1);
            f
        };
        let session = Session { iface, control, alive, stop };
        let ready_by = Instant::now() + Duration::from_secs(5);
        let mut sent = Instant::now() - Duration::from_secs(10);
        while !file_server.load(Ordering::SeqCst) {
            if Instant::now() >= ready_by || !session.is_alive() {
                return Err(Error::Session("the transmitter didn't start its file server".into()));
            }
            if sent.elapsed() >= Duration::from_secs(1) {
                (&session.control).write_all(&start).map_err(|e| Error::Session(e.to_string()))?;
                sent = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(session)
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// The recordings on the transmitter.
    pub fn list(&self) -> Result<Vec<RemoteFile>, Error> {
        let (status, _, body) = self.get(FILES_INFO, None).map_err(|e| Error::Session(e.to_string()))?;
        let mut body_bytes = Vec::new();
        body.take(4 << 20).read_to_end(&mut body_bytes).map_err(|e| Error::Session(e.to_string()))?;
        if status != 200 {
            return Err(Error::Session(format!("file list: HTTP {status}")));
        }
        let info: FilesInfo = serde_json::from_slice(&body_bytes).map_err(|e| Error::Session(format!("file list: {e}")))?;
        Ok(info.files)
    }

    /// The first `len` bytes of a file (enough for its WAV header).
    pub fn head(&self, fname: &str, len: u64) -> io::Result<Vec<u8>> {
        let (status, _, body) = self.get(fname, Some((0, Some(len.saturating_sub(1)))))?;
        if status != 206 && status != 200 {
            return Err(io::Error::other(format!("HTTP {status}")));
        }
        let mut out = Vec::new();
        body.take(len).read_to_end(&mut out)?;
        Ok(out)
    }

    /// Download `fname` (`size` bytes) to `part`, resuming what's already
    /// there. `progress` gets the bytes written so far.
    pub fn download(&self, fname: &str, size: u64, part: &Path, cancel: &AtomicBool, progress: &mut dyn FnMut(u64)) -> io::Result<()> {
        let have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
        let have = if have > size { 0 } else { have };
        if have == size && size > 0 {
            progress(size);
            return Ok(());
        }
        let (status, headers, mut body) = self.get(fname, (have > 0).then_some((have, None)))?;
        let mut file = match status {
            206 => OpenOptions::new().append(true).open(part)?,
            200 => File::create(part)?,
            s => return Err(io::Error::other(format!("HTTP {s}"))),
        };
        let mut done = if status == 206 { have } else { 0 };
        let expected = headers.get("content-length").and_then(|v| v.parse::<u64>().ok()).map(|n| done + n).unwrap_or(size);
        if expected != size {
            return Err(io::Error::other(format!("server reports {expected} bytes, expected {size}")));
        }
        let mut buf = vec![0u8; 256 * 1024];
        while done < size {
            if cancel.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            let want = buf.len().min((size - done) as usize);
            let n = body.read(&mut buf[..want])?;
            if n == 0 {
                return Err(io::Error::new(io::ErrorKind::UnexpectedEof, format!("connection closed at {done} of {size} bytes")));
            }
            file.write_all(&buf[..n])?;
            done += n as u64;
            progress(done);
        }
        file.sync_all()
    }

    /// Minimal HTTP/1.1 GET with an optional byte range.
    fn get(&self, path: &str, range: Option<(u64, Option<u64>)>) -> io::Result<(u16, HashMap<String, String>, BufReader<TcpStream>)> {
        let stream = self.iface.connect(HTTP_PORT, Duration::from_secs(5))?;
        stream.set_read_timeout(Some(Duration::from_secs(15)))?;
        let mut req = format!("GET /{} HTTP/1.1\r\nhost: {}\r\nconnection: close\r\n", path.trim_start_matches('/'), link::TX_ADDR);
        if let Some((from, to)) = range {
            req += &format!("range: bytes={from}-{}\r\n", to.map(|t| t.to_string()).unwrap_or_default());
        }
        req += "\r\n";
        (&stream).write_all(req.as_bytes())?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let status = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or_else(|| io::Error::other(format!("bad HTTP status line: {line:?}")))?;
        let mut headers = HashMap::new();
        loop {
            line.clear();
            if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }
        Ok((status, headers, reader))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = self.control.shutdown(std::net::Shutdown::Both);
    }
}
