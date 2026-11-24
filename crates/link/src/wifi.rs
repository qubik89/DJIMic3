//! Joining the transmitter's access point and going back afterwards, with
//! each OS's own tools: `networksetup` on macOS, `netsh wlan` on Windows,
//! NetworkManager's `nmcli` on Linux.

#[cfg(not(target_os = "macos"))]
use std::net::Ipv4Addr;
use std::process::Command;
use std::time::{Duration, Instant};

use super::net::Iface;
use super::Error;

/// What to undo when leaving.
#[derive(Debug, Clone)]
pub struct Link {
    pub iface: Iface,
    ssid: String,
    /// The network or profile to go back to, where the OS needs telling.
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    previous: Option<String>,
    /// Wi-Fi was switched on by us (macOS turns it back off).
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    powered_on: bool,
}

fn run(cmd: &mut Command) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    if out.status.success() {
        Ok(stdout + &stderr)
    } else {
        let msg = if stderr.trim().is_empty() { stdout } else { stderr };
        Err(msg.trim().to_string())
    }
}

/// Join `ssid` only if it's on the air right now (one quick attempt): for a
/// unit whose access point is still up from an earlier session.
pub fn join_if_present(ssid: &str, psk: &str) -> Option<Link> {
    join(ssid, psk, Duration::from_secs(3)).ok()
}

// ---- macOS -------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn wifi_device() -> Option<String> {
    let out = run(Command::new("/usr/sbin/networksetup").arg("-listallhardwareports")).ok()?;
    let mut lines = out.lines();
    while let Some(l) = lines.next() {
        if l.trim() == "Hardware Port: Wi-Fi" || l.trim() == "Hardware Port: AirPort" {
            return lines.next()?.trim().strip_prefix("Device: ").map(str::to_string);
        }
    }
    None
}

#[cfg(target_os = "macos")]
pub fn join(ssid: &str, psk: &str, timeout: Duration) -> Result<Link, Error> {
    let ns = "/usr/sbin/networksetup";
    let dev = wifi_device().ok_or_else(|| Error::Wifi("this Mac has no Wi-Fi".into()))?;
    let power = run(Command::new(ns).args(["-getairportpower", &dev])).unwrap_or_default();
    let powered_on = power.trim_end().ends_with("Off");
    if powered_on {
        run(Command::new(ns).args(["-setairportpower", &dev, "on"])).map_err(Error::Wifi)?;
    }
    // The access point takes a few seconds to show up after the BLE link
    // drops; networksetup prints an error (with exit status 0) until then.
    let deadline = Instant::now() + timeout;
    let mut last = String::new();
    while Instant::now() < deadline {
        match run(Command::new(ns).args(["-setairportnetwork", &dev, ssid, psk])) {
            Ok(out) if out.trim().is_empty() => {
                return Ok(Link { iface: Iface { name: dev, local_ip: None }, ssid: ssid.into(), previous: None, powered_on });
            }
            Ok(out) | Err(out) => last = out,
        }
        std::thread::sleep(Duration::from_millis(800));
    }
    Err(Error::Wifi(if last.is_empty() { format!("couldn't join {ssid}") } else { last }))
}

/// Drop saved networks of DJI Mic units left behind by a session that
/// ended abruptly (app killed), so the computer never rejoins one by itself.
#[cfg(target_os = "macos")]
pub fn forget_unit_networks() {
    let ns = "/usr/sbin/networksetup";
    let Some(dev) = wifi_device() else { return };
    let list = run(Command::new(ns).args(["-listpreferredwirelessnetworks", &dev])).unwrap_or_default();
    for name in list.lines().map(str::trim).filter(|n| n.starts_with("DJI Mic 3 ")) {
        let _ = run(Command::new(ns).args(["-removepreferredwirelessnetwork", &dev, name]));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn forget_unit_networks() {}

#[cfg(target_os = "macos")]
pub fn leave(link: &Link) {
    let ns = "/usr/sbin/networksetup";
    let dev = &link.iface.name;
    let _ = run(Command::new(ns).args(["-removepreferredwirelessnetwork", dev, &link.ssid]));
    // Cycling the radio makes macOS rejoin its usual network straight away.
    let _ = run(Command::new(ns).args(["-setairportpower", dev, "off"]));
    if !link.powered_on {
        let _ = run(Command::new(ns).args(["-setairportpower", dev, "on"]));
    }
}

// ---- Windows -----------------------------------------------------------------

#[cfg(windows)]
fn field(out: &str, key: &str) -> Option<String> {
    out.lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim().to_string())
}

#[cfg(windows)]
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

#[cfg(windows)]
pub fn join(ssid: &str, psk: &str, timeout: Duration) -> Result<Link, Error> {
    let shown = run(Command::new("netsh").args(["wlan", "show", "interfaces"])).map_err(Error::Wifi)?;
    let name = field(&shown, "Name").ok_or_else(|| Error::Wifi("no Wi-Fi adapter".into()))?;
    let previous = field(&shown, "Profile");
    let profile = format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
  <name>{n}</name>
  <SSIDConfig><SSID><name>{n}</name></SSID></SSIDConfig>
  <connectionType>ESS</connectionType>
  <connectionMode>manual</connectionMode>
  <MSM><security>
    <authEncryption><authentication>WPA2PSK</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption>
    <sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{k}</keyMaterial></sharedKey>
  </security></MSM>
</WLANProfile>"#,
        n = xml_escape(ssid),
        k = xml_escape(psk)
    );
    let file = std::env::temp_dir().join("djimic-tx-wifi.xml");
    std::fs::write(&file, profile).map_err(|e| Error::Wifi(e.to_string()))?;
    let added = run(Command::new("netsh").args(["wlan", "add", "profile"]).arg(format!("filename={}", file.display())).arg("user=current"));
    let _ = std::fs::remove_file(&file);
    added.map_err(Error::Wifi)?;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        let _ = run(Command::new("netsh").args(["wlan", "connect"]).arg(format!("name={ssid}")).arg(format!("interface={name}")));
        std::thread::sleep(Duration::from_millis(1500));
        let now = run(Command::new("netsh").args(["wlan", "show", "interfaces"])).unwrap_or_default();
        if field(&now, "SSID").as_deref() == Some(ssid) && field(&now, "State").is_some_and(|s| s.eq_ignore_ascii_case("connected")) {
            let local_ip = local_ipv4(&name);
            return Ok(Link { iface: Iface { name, local_ip }, ssid: ssid.into(), previous, powered_on: false });
        }
    }
    let _ = run(Command::new("netsh").args(["wlan", "delete", "profile"]).arg(format!("name={ssid}")));
    Err(Error::Wifi(format!("couldn't join {ssid}")))
}

#[cfg(windows)]
fn local_ipv4(iface: &str) -> Option<Ipv4Addr> {
    for _ in 0..20 {
        let out = run(Command::new("netsh").args(["interface", "ipv4", "show", "addresses"]).arg(format!("name={iface}"))).unwrap_or_default();
        if let Some(ip) = out
            .lines()
            .filter(|l| l.contains("IP"))
            .filter_map(|l| l.split_whitespace().last()?.parse::<Ipv4Addr>().ok())
            .find(|ip| ip.octets()[..3] == [192, 168, 0])
        {
            return Some(ip);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}

#[cfg(windows)]
pub fn leave(link: &Link) {
    let _ = run(Command::new("netsh").args(["wlan", "disconnect"]).arg(format!("interface={}", link.iface.name)));
    let _ = run(Command::new("netsh").args(["wlan", "delete", "profile"]).arg(format!("name={}", link.ssid)));
    if let Some(prev) = &link.previous {
        let _ = run(Command::new("netsh").args(["wlan", "connect"]).arg(format!("name={prev}")).arg(format!("interface={}", link.iface.name)));
    }
}

// ---- Linux -------------------------------------------------------------------

#[cfg(all(unix, not(target_os = "macos")))]
const NM_NAME: &str = "djimic-transmitter";

#[cfg(all(unix, not(target_os = "macos")))]
pub fn join(ssid: &str, psk: &str, timeout: Duration) -> Result<Link, Error> {
    let devs = run(Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "device"]))
        .map_err(|e| Error::Wifi(format!("NetworkManager (nmcli) is needed: {e}")))?;
    let dev = devs
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(_, t)| *t == "wifi")
        .map(|(d, _)| d.to_string())
        .ok_or_else(|| Error::Wifi("no Wi-Fi adapter".into()))?;
    let active = run(Command::new("nmcli").args(["-t", "-f", "NAME,DEVICE", "connection", "show", "--active"])).unwrap_or_default();
    let previous = active.lines().filter_map(|l| l.rsplit_once(':')).find(|(_, d)| *d == dev).map(|(n, _)| n.replace("\\:", ":"));
    let _ = run(Command::new("nmcli").args(["connection", "delete", NM_NAME]));
    let deadline = Instant::now() + timeout;
    let mut last = String::new();
    while Instant::now() < deadline {
        let _ = run(Command::new("nmcli").args(["device", "wifi", "rescan", "ifname", &dev]));
        match run(Command::new("nmcli").args(["device", "wifi", "connect", ssid, "password", psk, "ifname", &dev, "name", NM_NAME])) {
            Ok(_) => {
                let local_ip = local_ipv4(&dev);
                return Ok(Link { iface: Iface { name: dev, local_ip }, ssid: ssid.into(), previous, powered_on: false });
            }
            Err(e) => last = e,
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Err(Error::Wifi(last))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn local_ipv4(dev: &str) -> Option<Ipv4Addr> {
    let out = run(Command::new("nmcli").args(["-t", "-f", "IP4.ADDRESS", "device", "show", dev])).ok()?;
    out.lines().filter_map(|l| l.split_once(':')).filter_map(|(_, v)| v.split('/').next()?.parse().ok()).next()
}

#[cfg(all(unix, not(target_os = "macos")))]
pub fn leave(link: &Link) {
    let _ = run(Command::new("nmcli").args(["connection", "delete", NM_NAME]));
    if let Some(prev) = &link.previous {
        let _ = run(Command::new("nmcli").args(["connection", "up", prev]));
    }
}
