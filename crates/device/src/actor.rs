//! Per-device background task.
//!
//! Each connected device is driven by one thread that owns its link: a USB
//! interface, or a TCP connection to a receiver's own Wi-Fi access point
//! (the same DUML frames either way). The thread continuously reads frames
//! (decoding heartbeats into a shared [`DeviceStatus`]) and, interleaved,
//! writes command packets requested over a channel. Because `nusb` transfers
//! are queued rather than blocking, reads and writes coexist without the
//! global lock a synchronous USB API needs.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures_lite::future;
use nusb::transfer::{Bulk, Completion, In, Out};
use nusb::{Endpoint, Interface};
use protocol::packet::Dialect;
use protocol::{packet, DeviceModel, DeviceStatus};

use crate::error::{DeviceError, Result};

fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x} ")).collect()
}

/// A request to change one setting, with a channel to report the outcome.
pub struct Command {
    pub setting_id: String,
    pub value: String,
    /// The 0-based transmitter slot to address, for settings that target one
    /// specific TX rather than broadcasting (see `V2Target::Tx`). `None` for
    /// every other setting.
    pub tx: Option<usize>,
    pub resp: std::sync::mpsc::Sender<Result<()>>,
}

/// The channels and shared state a running device exposes to the manager.
pub struct Actor {
    pub cmd_tx: async_channel::Sender<Command>,
    pub status: Arc<Mutex<DeviceStatus>>,
    pub alive: Arc<AtomicBool>,
    /// The model currently driving this device. Starts as the one matched by
    /// USB id and may be replaced once the receiver identifies itself (see
    /// `DeviceModel::refine`).
    pub model: Arc<Mutex<&'static dyn DeviceModel>>,
    /// Unix-millis timestamp of the last decoded heartbeat (0 = none yet).
    pub last_seen: Arc<AtomicU64>,
}

/// Milliseconds since the Unix epoch.
pub fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// How many bulk-IN reads to wait for a command's ACK before re-sending it.
/// The device pushes status at least twice a second even with no transmitter
/// connected, so this bounds the wait to a second or two.
const ACK_WAIT_READS: u8 = 3;
/// How many times an unacknowledged command is re-sent before giving up.
const ACK_RETRIES: u8 = 2;

/// A command written to the device whose ACK hasn't arrived yet.
struct Pending {
    seq: u16,
    packet: Vec<u8>,
    reads_left: u8,
    attempts_left: u8,
    resp: std::sync::mpsc::Sender<Result<()>>,
}

/// Bulk-IN transfers kept queued at all times.
const READS_IN_FLIGHT: usize = 4;

/// What a device is reached through.
pub enum Transport {
    /// A claimed USB interface.
    Usb(Interface),
    /// The DUML session on a receiver's access point (TCP port 60000).
    Tcp(TcpStream),
}

/// A receiver pushes status twice a second; this much silence on TCP means
/// its access point is gone.
const TCP_SILENCE: Duration = Duration::from_secs(5);

/// The open link, with one read/write interface for the event loop.
enum Link {
    Usb {
        ep_in: Endpoint<Bulk, In>,
        ep_out: Endpoint<Bulk, Out>,
    },
    Tcp {
        rx: async_channel::Receiver<Vec<u8>>,
        writer: Arc<Mutex<TcpStream>>,
        stop: Arc<AtomicBool>,
    },
}

/// A DUML request to any address (the keepalive's heartbeat goes to `0xf0`).
fn request_to(seq: u16, dst: u8, cmd_set: u8, cmd_id: u8, payload: &[u8]) -> Vec<u8> {
    let len = 13 + payload.len();
    let mut body = vec![packet::SOF, len as u8, 0x04, 0];
    body[3] = packet::header_crc8(&body[..3]);
    body.extend_from_slice(&[0x02, dst, seq as u8, (seq >> 8) as u8, 0x40, cmd_set, cmd_id]);
    body.extend_from_slice(payload);
    protocol::crc::append(body, Dialect::V2.crc_init(), Dialect::V2.crc_residue())
}

impl Link {
    fn open(model: &'static dyn DeviceModel, transport: Transport) -> Option<Link> {
        match transport {
            Transport::Usb(iface) => match (iface.endpoint::<Bulk, In>(model.bulk_in()), iface.endpoint::<Bulk, Out>(model.bulk_out())) {
                (Ok(ep_in), Ok(ep_out)) => Some(Link::Usb { ep_in, ep_out }),
                _ => None, // endpoints missing: not the interface we expected
            },
            Transport::Tcp(stream) => {
                let mut reader = stream.try_clone().ok()?;
                reader.set_read_timeout(Some(TCP_SILENCE)).ok()?;
                let writer = Arc::new(Mutex::new(stream));
                let stop = Arc::new(AtomicBool::new(false));
                let (tx, rx) = async_channel::unbounded();
                // Reader: hand chunks to the event loop; EOF, an error or
                // silence closes the channel, which ends the actor.
                std::thread::spawn(move || {
                    let mut buf = [0u8; 4096];
                    while let Ok(n) = reader.read(&mut buf) {
                        if n == 0 || tx.send_blocking(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                });
                // Keepalive, as DJI Mimo does it: ping (00/00) every 0.5 s and
                // the app heartbeat (00/2b to 0xf0) every 2.5 s. Its own
                // sequence range keeps the answers clear of command ACKs.
                let (w, s) = (writer.clone(), stop.clone());
                std::thread::spawn(move || {
                    let mut seq: u16 = 0xc000;
                    let mut n = 0u32;
                    while !s.load(Ordering::Relaxed) {
                        let mut frames = vec![request_to(seq, 0x5a, 0x00, 0x00, &[0x01, 0x00])];
                        if n.is_multiple_of(5) {
                            frames.push(request_to(seq.wrapping_add(1), 0xf0, 0x00, 0x2b, &[0x04, 0x00]));
                        }
                        seq = if seq >= 0xfff0 { 0xc000 } else { seq + 2 };
                        let mut out = w.lock().unwrap();
                        if frames.iter().any(|f| out.write_all(f).is_err()) {
                            break;
                        }
                        drop(out);
                        n += 1;
                        std::thread::sleep(Duration::from_millis(500));
                    }
                });
                Some(Link::Tcp { rx, writer, stop })
            }
        }
    }

    /// The next chunk of bytes, or `None` once the link is gone.
    async fn read(&mut self) -> Option<Vec<u8>> {
        match self {
            Link::Usb { ep_in, .. } => {
                // Keep a few reads queued so status frames are never missed
                // while a command is being handled.
                while ep_in.pending() < READS_IN_FLIGHT {
                    let buffer = ep_in.allocate(512);
                    ep_in.submit(buffer);
                }
                let comp: Completion = ep_in.next_complete().await;
                comp.status.is_ok().then(|| comp.buffer.to_vec())
            }
            Link::Tcp { rx, .. } => rx.recv().await.ok(),
        }
    }

    /// Send one packet.
    async fn write(&mut self, pkt: Vec<u8>) -> std::result::Result<(), String> {
        match self {
            Link::Usb { ep_out, .. } => {
                ep_out.submit(pkt.into());
                ep_out.next_complete().await.status.map_err(|e| format!("{e:?}"))
            }
            Link::Tcp { writer, .. } => writer.lock().unwrap().write_all(&pkt).map_err(|e| e.to_string()),
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        if let Link::Tcp { writer, stop, .. } = self {
            stop.store(true, Ordering::Relaxed);
            let _ = writer.lock().unwrap().shutdown(Shutdown::Both);
        }
    }
}

/// What the event loop woke up for.
enum Wake {
    Read(Option<Vec<u8>>),
    Command(std::result::Result<Command, async_channel::RecvError>),
}

/// Spawn the background thread for a device and return its handles.
pub fn spawn(model: &'static dyn DeviceModel, transport: Transport) -> Actor {
    let (cmd_tx, cmd_rx) = async_channel::unbounded::<Command>();
    let status = Arc::new(Mutex::new(DeviceStatus::disconnected(model.id())));
    let alive = Arc::new(AtomicBool::new(true));
    let last_seen = Arc::new(AtomicU64::new(0));
    let model_cell = Arc::new(Mutex::new(model));

    let status_thread = status.clone();
    let model_thread = model_cell.clone();
    let alive_thread = alive.clone();
    let last_seen_thread = last_seen.clone();
    std::thread::spawn(move || {
        future::block_on(run(model_thread, transport, cmd_rx, status_thread, last_seen_thread));
        alive_thread.store(false, Ordering::Relaxed);
    });

    Actor { cmd_tx, status, alive, last_seen, model: model_cell }
}

async fn run(
    model_cell: Arc<Mutex<&'static dyn DeviceModel>>,
    transport: Transport,
    cmd_rx: async_channel::Receiver<Command>,
    status: Arc<Mutex<DeviceStatus>>,
    last_seen: Arc<AtomicU64>,
) {
    let mut model = *model_cell.lock().unwrap();
    let Some(mut link) = Link::open(model, transport) else { return };
    let mut seq: u16 = 1;
    let mut buf: Vec<u8> = Vec::with_capacity(1024);
    let debug = std::env::var_os("DJIMIC_DEBUG").is_some();
    // Learned from the first heartbeat; defaults to v2 since a command sent
    // before any heartbeat arrives is the rare case (heartbeats stream
    // continuously as soon as the device is opened).
    let mut dialect = Dialect::V2;

    // The command currently awaiting its ACK, if any. While one is in flight
    // no further command is accepted: the device answers each with a 14-byte
    // ACK echoing its sequence number (see `PROTOCOL.md`), carrying a result
    // code at offset 11 (`0x00` = applied). Some receivers silently drop the
    // first command sent after the interface is opened, so an unacknowledged
    // command is re-sent a few times before giving up.
    let mut pending: Option<Pending> = None;
    // One-shot packets the model wants sent once the device is streaming
    // (e.g. a clock sync). Queued on the first decoded status and sent like
    // commands, ACK and retries included, with nobody waiting on the result.
    let mut connect_queue: Vec<Vec<u8>> = Vec::new();
    let mut connected_once = false;

    loop {
        let wake = if pending.is_some() {
            Wake::Read(link.read().await)
        } else {
            // Borrow the link only inside the race so a command can be
            // written right after.
            let link_ref = &mut link;
            future::or(async { Wake::Read(link_ref.read().await) }, async { Wake::Command(cmd_rx.recv().await) }).await
        };

        match wake {
            Wake::Read(data) => {
                if debug {
                    eprintln!("[read] {}", data.as_deref().map(hex).unwrap_or_else(|| "link closed".into()));
                }
                let Some(data) = data else {
                    // Device unplugged or fatal transfer error. A command
                    // that was written but not yet acknowledged may be the
                    // very thing that made the device re-enumerate (e.g. a
                    // channel-mode change), so report it as sent.
                    if let Some(p) = pending.take() {
                        let _ = p.resp.send(Ok(()));
                    }
                    break;
                };
                buf.extend_from_slice(&data);
                while let Some(frame) = packet::take_frame(&mut buf) {
                    if let Some(d) = packet::heartbeat_dialect(&frame) {
                        dialect = d;
                    }
                    // The "get version" answer names the receiver's hardware.
                    if let Some(name) = packet::parse_version_response(&frame) {
                        let mut st = status.lock().unwrap();
                        st.rx.get_or_insert_with(protocol::RxInfo::unknown).hardware = Some(name);
                    }
                    if packet::frame_kind(&frame) == packet::FrameKind::Ack {
                        let ack_seq = u16::from_le_bytes([frame[6], frame[7]]);
                        if pending.as_ref().is_some_and(|p| p.seq == ack_seq) {
                            let p = pending.take().unwrap();
                            let code = frame.get(11).copied().unwrap_or(0);
                            let _ = p.resp.send(if code == 0 {
                                Ok(())
                            } else {
                                Err(DeviceError::Rejected(code))
                            });
                        }
                    }
                    let previous = status.lock().unwrap().clone();
                    let new_status = model.decode(&previous, &frame);
                    if debug {
                        eprintln!(
                            "[frame] kind={:?} decoded={} {}",
                            packet::frame_kind(&frame),
                            new_status.is_some(),
                            hex(&frame),
                        );
                    }
                    if let Some(mut new_status) = new_status {
                        // Switch to the model that fits once the receiver
                        // has identified itself. Models that share a USB id
                        // share the transport too, so the open interface
                        // stays valid.
                        if let Some(refined) = model.refine(&new_status) {
                            if refined.id() != model.id() {
                                if debug {
                                    eprintln!("[model] {} -> {}", model.id(), refined.id());
                                }
                                model = refined;
                                *model_cell.lock().unwrap() = refined;
                                new_status.model_id = refined.id().to_string();
                            }
                        }
                        *status.lock().unwrap() = new_status;
                        last_seen.store(now_millis(), Ordering::Relaxed);
                        if !connected_once {
                            connected_once = true;
                            connect_queue = model.connect_commands(seq, dialect);
                            seq = seq.wrapping_add(connect_queue.len() as u16);
                            connect_queue.reverse();
                        }
                    }
                }
                if pending.is_none() {
                    if let Some(pkt) = connect_queue.pop() {
                        let pkt_seq = u16::from_le_bytes([pkt[6], pkt[7]]);
                        let (resp, _) = std::sync::mpsc::channel();
                        if link.write(pkt.clone()).await.is_ok() {
                            pending = Some(Pending {
                                seq: pkt_seq,
                                packet: pkt,
                                reads_left: ACK_WAIT_READS,
                                attempts_left: ACK_RETRIES,
                                resp,
                            });
                        }
                        continue;
                    }
                }
                // No ACK among the frames of this read: count it against the
                // pending command, re-sending or giving up as it runs out.
                if let Some(p) = pending.as_mut() {
                    p.reads_left -= 1;
                    if p.reads_left == 0 {
                        if p.attempts_left == 0 {
                            let p = pending.take().unwrap();
                            let _ = p.resp.send(Err(DeviceError::NoAck));
                        } else {
                            p.attempts_left -= 1;
                            p.reads_left = ACK_WAIT_READS;
                            if let Err(e) = link.write(p.packet.clone()).await {
                                let p = pending.take().unwrap();
                                let _ = p.resp.send(Err(DeviceError::Usb(e)));
                            }
                        }
                    }
                }
            }
            Wake::Command(Ok(cmd)) => {
                match model.build_command(seq, dialect, &cmd.setting_id, &cmd.value, cmd.tx) {
                    Some(pkt) => {
                        let sent_seq = seq;
                        seq = seq.wrapping_add(1);
                        match link.write(pkt.clone()).await {
                            Ok(()) => {
                                pending = Some(Pending {
                                    seq: sent_seq,
                                    packet: pkt,
                                    reads_left: ACK_WAIT_READS,
                                    attempts_left: ACK_RETRIES,
                                    resp: cmd.resp,
                                });
                            }
                            Err(e) => {
                                let _ = cmd.resp.send(Err(DeviceError::Usb(e)));
                            }
                        }
                    }
                    None => {
                        let _ = cmd
                            .resp
                            .send(Err(DeviceError::UnknownSetting(cmd.setting_id.clone())));
                    }
                }
            }
            Wake::Command(Err(_)) => break, // manager dropped the sender
        }
    }
}
