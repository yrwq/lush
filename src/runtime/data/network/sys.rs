use std::fs;

pub(super) fn read_totals(iface_filter: Option<&str>) -> Result<(u64, u64), String> {
    let raw = fs::read_to_string("/proc/net/dev").map_err(|e| e.to_string())?;
    let mut down_total: u64 = 0;
    let mut up_total: u64 = 0;
    let mut matched = false;

    for line in raw.lines().skip(2) {
        let Some((iface_raw, rest)) = line.split_once(':') else {
            continue;
        };
        let iface = iface_raw.trim();
        if let Some(filter) = iface_filter {
            if iface != filter {
                continue;
            }
        } else if iface == "lo" {
            continue;
        }

        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() < 16 {
            continue;
        }

        let rx_bytes = fields[0].parse::<u64>().map_err(|e| e.to_string())?;
        let tx_bytes = fields[8].parse::<u64>().map_err(|e| e.to_string())?;
        down_total = down_total.saturating_add(rx_bytes);
        up_total = up_total.saturating_add(tx_bytes);
        matched = true;
    }

    if iface_filter.is_some() && !matched {
        return Err(format!(
            "network interface '{}' not found in /proc/net/dev",
            iface_filter.unwrap_or_default()
        ));
    }

    Ok((down_total, up_total))
}

pub(super) fn default_route_iface() -> Option<String> {
    let raw = fs::read_to_string("/proc/net/route").ok()?;
    for line in raw.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 11 {
            continue;
        }
        if cols[1] == "00000000" {
            return Some(cols[0].to_string());
        }
    }
    None
}
