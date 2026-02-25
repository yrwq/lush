use std::ffi::CString;
use std::fs;

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

pub(super) fn wireless_iface_stats(
    iface_filter: Option<&str>,
) -> Result<Option<(String, f64, f64)>, String> {
    let raw = fs::read_to_string("/proc/net/wireless").map_err(|e| e.to_string())?;
    for line in raw.lines().skip(2) {
        let Some((iface_raw, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface_raw.trim().to_string();
        if let Some(filter) = iface_filter
            && iface != filter
        {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 3 {
            continue;
        }
        let quality = fields[1]
            .trim_end_matches('.')
            .parse::<f64>()
            .unwrap_or(0.0);
        let signal_dbm = fields[2]
            .trim_end_matches('.')
            .parse::<f64>()
            .unwrap_or(0.0);
        return Ok(Some((iface, quality, signal_dbm)));
    }
    Ok(None)
}

#[repr(C)]
#[derive(Clone, Copy)]
struct IwPoint {
    pointer: *mut libc::c_void,
    length: u16,
    flags: u16,
}

#[repr(C)]
union IwreqData {
    essid: IwPoint,
}

#[repr(C)]
struct Iwreq {
    ifr_name: [libc::c_char; libc::IFNAMSIZ],
    u: IwreqData,
}

const SIOCGIWESSID: libc::c_ulong = 0x8B1B;

pub(super) fn read_ssid(iface: &str) -> Option<String> {
    let ifname = CString::new(iface).ok()?;
    let mut req = Iwreq {
        ifr_name: [0; libc::IFNAMSIZ],
        u: IwreqData {
            essid: IwPoint {
                pointer: std::ptr::null_mut(),
                length: 0,
                flags: 0,
            },
        },
    };

    let name_bytes = ifname.as_bytes_with_nul();
    let max = (libc::IFNAMSIZ - 1).min(name_bytes.len().saturating_sub(1));
    for (i, b) in name_bytes.iter().take(max).enumerate() {
        req.ifr_name[i] = *b as libc::c_char;
    }

    let mut essid = [0_u8; 33];
    req.u = IwreqData {
        essid: IwPoint {
            pointer: essid.as_mut_ptr() as *mut libc::c_void,
            length: (essid.len() - 1) as u16,
            flags: 0,
        },
    };

    // socket call returns an owned fd on success.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return None;
    }

    // req points to initialized memory.
    let rc = unsafe { libc::ioctl(fd, SIOCGIWESSID, &mut req) };
    unsafe { libc::close(fd) };
    if rc < 0 {
        return None;
    }

    // ioctl has written req.u.essid.length when rc >= 0.
    let len = unsafe { req.u.essid.length as usize }.min(essid.len());
    let mut out = String::from_utf8_lossy(&essid[..len]).to_string();
    while out.ends_with('\0') {
        out.pop();
    }
    if out.is_empty() { None } else { Some(out) }
}

pub(super) fn nm_ssid_strength(iface_filter: Option<&str>) -> Option<(String, String, u8)> {
    let conn = Connection::system().ok()?;
    let nm = Proxy::new(
        &conn,
        "org.freedesktop.NetworkManager",
        "/org/freedesktop/NetworkManager",
        "org.freedesktop.NetworkManager",
    )
    .ok()?;

    if let Some(iface) = iface_filter {
        let dev_path: OwnedObjectPath = nm.call("GetDeviceByIpIface", &(iface)).ok()?;
        if dev_path.as_str() == "/" {
            return None;
        }

        let dev = Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            dev_path.as_str(),
            "org.freedesktop.NetworkManager.Device.Wireless",
        )
        .ok()?;

        let ap_path: OwnedObjectPath = dev.get_property("ActiveAccessPoint").ok()?;
        if ap_path.as_str() == "/" {
            return None;
        }

        let ap = Proxy::new(
            &conn,
            "org.freedesktop.NetworkManager",
            ap_path.as_str(),
            "org.freedesktop.NetworkManager.AccessPoint",
        )
        .ok()?;

        let ssid_bytes: Vec<u8> = ap.get_property("Ssid").ok()?;
        let strength: u8 = ap.get_property("Strength").ok()?;
        let ssid = String::from_utf8_lossy(&ssid_bytes).trim().to_string();
        if ssid.is_empty() {
            None
        } else {
            Some((iface.to_string(), ssid, strength))
        }
    } else {
        let devices: Vec<OwnedObjectPath> = nm.call("GetDevices", &()).ok()?;
        for dev_path in devices {
            let dev = Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                dev_path.as_str(),
                "org.freedesktop.NetworkManager.Device",
            )
            .ok()?;
            let device_type: u32 = dev.get_property("DeviceType").ok()?;
            if device_type != 2 {
                continue;
            }
            let iface: String = dev.get_property("Interface").ok()?;

            let wifi = Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                dev_path.as_str(),
                "org.freedesktop.NetworkManager.Device.Wireless",
            )
            .ok()?;
            let ap_path: OwnedObjectPath = wifi.get_property("ActiveAccessPoint").ok()?;
            if ap_path.as_str() == "/" {
                continue;
            }

            let ap = Proxy::new(
                &conn,
                "org.freedesktop.NetworkManager",
                ap_path.as_str(),
                "org.freedesktop.NetworkManager.AccessPoint",
            )
            .ok()?;
            let ssid_bytes: Vec<u8> = ap.get_property("Ssid").ok()?;
            let strength: u8 = ap.get_property("Strength").ok()?;
            let ssid = String::from_utf8_lossy(&ssid_bytes).trim().to_string();
            if !ssid.is_empty() {
                return Some((iface, ssid, strength));
            }
        }
        None
    }
}
