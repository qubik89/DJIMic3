//! DUML frames with any source, destination and flags — the same framing the
//! USB transport uses, but the wireless link talks to the transmitter
//! (`0x3a`) and needs responses as well as requests.

use protocol::crc;
use protocol::packet::{header_crc8, Dialect};

/// The app's address.
pub const APP: u8 = 0x02;
/// A Mic 3 transmitter's address.
pub const TX: u8 = 0x3a;
/// A Mic 3 receiver's address.
pub const RX: u8 = 0x5a;
/// Where DJI Mimo sends its app heartbeat.
pub const BROADCAST: u8 = 0xf0;
/// Request, answer wanted.
pub const REQUEST: u8 = 0x40;
/// Set on every answer.
pub const RESPONSE: u8 = 0x80;

/// A decoded frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub src: u8,
    pub dst: u8,
    pub seq: u16,
    pub flags: u8,
    pub set: u8,
    pub id: u8,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn is(&self, set: u8, id: u8) -> bool {
        self.set == set && self.id == id
    }

    pub fn is_response(&self) -> bool {
        self.flags & RESPONSE != 0
    }
}

/// Encode one frame. Lengths above 255 bytes spill into the low two bits of
/// the version byte.
pub fn encode(seq: u16, src: u8, dst: u8, flags: u8, set: u8, id: u8, payload: &[u8]) -> Vec<u8> {
    let len = 13 + payload.len();
    let mut body = vec![0x55, len as u8, 0x04 | ((len >> 8) as u8 & 0x03), 0];
    body[3] = header_crc8(&body[..3]);
    body.extend_from_slice(&[src, dst, seq as u8, (seq >> 8) as u8, flags, set, id]);
    body.extend_from_slice(payload);
    crc::append(body, Dialect::V2.crc_init(), Dialect::V2.crc_residue())
}

/// Split a byte stream (BLE notifications, TCP) into frames.
#[derive(Default)]
pub struct Reassembler {
    buf: Vec<u8>,
}

impl Reassembler {
    pub fn push(&mut self, data: &[u8]) -> Vec<Frame> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        loop {
            let Some(i) = self.buf.iter().position(|&b| b == 0x55) else {
                self.buf.clear();
                break;
            };
            self.buf.drain(..i);
            if self.buf.len() < 4 {
                break;
            }
            if self.buf[2] & 0xfc != 0x04 || header_crc8(&self.buf[..3]) != self.buf[3] {
                self.buf.remove(0);
                continue;
            }
            let len = self.buf[1] as usize | ((self.buf[2] as usize & 0x03) << 8);
            if len < 13 {
                self.buf.remove(0);
                continue;
            }
            if self.buf.len() < len {
                break;
            }
            let f: Vec<u8> = self.buf.drain(..len).collect();
            out.push(Frame {
                src: f[4],
                dst: f[5],
                seq: u16::from_le_bytes([f[6], f[7]]),
                flags: f[8],
                set: f[9],
                id: f[10],
                payload: f[11..len - 2].to_vec(),
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn encodes_what_mimo_sends() {
        // Captured from DJI Mimo / built by hand and answered by a Mic 3 TX.
        assert_eq!(encode(0x0101, APP, TX, REQUEST, 0x00, 0x01, &[]), unhex("550d0433023a0101400001871d"));
        assert_eq!(
            encode(0x0303, APP, TX, REQUEST, 0x07, 0x18, &[0x41, 0x44, 0, 0]),
            unhex("55110492023a030340071841440000f6cc")
        );
    }

    #[test]
    fn reassembles_split_and_long_frames() {
        let ssid = unhex("552204ea3a0224038007070013444a49204d696320332054582d3831376464383b3c");
        let mut r = Reassembler::default();
        assert!(r.push(&ssid[..10]).is_empty());
        let frames = r.push(&ssid[10..]);
        assert_eq!(frames.len(), 1);
        let f = &frames[0];
        assert_eq!((f.src, f.dst, f.set, f.id, f.is_response()), (TX, APP, 0x07, 0x07, true));
        assert_eq!(&f.payload[2..], b"DJI Mic 3 TX-817dd8");

        let big = encode(7, TX, APP, 0, 0x00, 0x24, &[0xab; 400]);
        assert_eq!(big[1] as usize | ((big[2] as usize & 3) << 8), big.len());
        let mut noisy = vec![0x55, 0x00, 0x13];
        noisy.extend_from_slice(&big);
        let frames = r.push(&noisy);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload.len(), 400);
    }
}
