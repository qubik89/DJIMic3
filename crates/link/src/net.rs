//! TCP connections pinned to the Wi-Fi interface.
//!
//! The transmitter's access point hands out 192.168.0.x addresses, the most
//! common home subnet. With the computer also on a wired 192.168.0.0/24
//! network, a plain connect to 192.168.0.1 would reach the home router, so
//! every socket is bound to the Wi-Fi interface.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, Type};

/// The transmitter's address on its own access point.
pub const TX_ADDR: Ipv4Addr = Ipv4Addr::new(192, 168, 0, 1);

/// The Wi-Fi interface joined to the transmitter.
#[derive(Debug, Clone)]
pub struct Iface {
    /// `en1`, `wlan0`, or on Windows the interface name (`Wi-Fi`).
    pub name: String,
    /// Our address on the access point, when known (needed on Windows).
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    pub local_ip: Option<Ipv4Addr>,
}

impl Iface {
    /// Connect to `port` on the transmitter through this interface.
    pub fn connect(&self, port: u16, timeout: Duration) -> io::Result<TcpStream> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        self.bind(&socket)?;
        let addr = SocketAddr::V4(SocketAddrV4::new(TX_ADDR, port));
        socket.connect_timeout(&addr.into(), timeout)?;
        let stream: TcpStream = socket.into();
        stream.set_nodelay(true)?;
        Ok(stream)
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn bind(&self, socket: &Socket) -> io::Result<()> {
        let name = std::ffi::CString::new(self.name.clone()).map_err(|_| io::Error::other("bad interface name"))?;
        // SAFETY: if_nametoindex reads a NUL-terminated string.
        let index = unsafe { libc::if_nametoindex(name.as_ptr()) };
        let index = std::num::NonZeroU32::new(index).ok_or_else(|| io::Error::other(format!("no interface {}", self.name)))?;
        socket.bind_device_by_index_v4(Some(index))
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn bind(&self, socket: &Socket) -> io::Result<()> {
        socket.bind_device(Some(self.name.as_bytes())).or_else(|e| match self.local_ip {
            // SO_BINDTODEVICE can need privileges on old kernels: fall back
            // to the source address.
            Some(ip) => socket.bind(&SocketAddr::V4(SocketAddrV4::new(ip, 0)).into()),
            None => Err(e),
        })
    }

    #[cfg(windows)]
    fn bind(&self, socket: &Socket) -> io::Result<()> {
        // Windows sends from the interface that owns the source address.
        match self.local_ip {
            Some(ip) => socket.bind(&SocketAddr::V4(SocketAddrV4::new(ip, 0)).into()),
            None => Ok(()),
        }
    }
}
