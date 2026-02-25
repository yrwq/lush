use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

const SWAY_IPC_MAGIC: &[u8; 6] = b"i3-ipc";
pub(super) const SWAY_IPC_RUN_COMMAND: u32 = 0;
pub(super) const SWAY_IPC_GET_WORKSPACES: u32 = 1;
pub(super) const SWAY_IPC_SUBSCRIBE: u32 = 2;
pub(super) const SWAY_IPC_GET_OUTPUTS: u32 = 3;
pub(super) const SWAY_IPC_GET_TREE: u32 = 4;

pub(super) fn socket_path() -> Option<String> {
    if let Ok(path) = std::env::var("SWAYSOCK")
        && !path.trim().is_empty()
    {
        return Some(path);
    }
    if let Ok(path) = std::env::var("I3SOCK")
        && !path.trim().is_empty()
    {
        return Some(path);
    }

    let runtime = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let dir = std::fs::read_dir(runtime).ok()?;
    let mut candidates: Vec<String> = dir
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("sway-ipc.") || !name.ends_with(".sock") {
                return None;
            }
            Some(entry.path().to_string_lossy().to_string())
        })
        .collect();
    candidates.sort();
    candidates.pop()
}

pub(super) fn subscribe_events<const N: usize>(events: [&str; N]) -> Option<UnixStream> {
    let payload = serde_json::to_string(&events.into_iter().collect::<Vec<_>>()).ok()?;
    let mut stream = UnixStream::connect(socket_path()?).ok()?;
    write_message(&mut stream, SWAY_IPC_SUBSCRIBE, payload.as_bytes()).ok()?;
    let (_, body) = read_message(&mut stream).ok()?;
    let reply: Value = serde_json::from_slice(&body).ok()?;
    if reply.get("success").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    Some(stream)
}

pub(super) fn request_json(msg_type: u32, payload: &str) -> Option<Value> {
    let mut stream = UnixStream::connect(socket_path()?).ok()?;
    write_message(&mut stream, msg_type, payload.as_bytes()).ok()?;
    let (_, body) = read_message(&mut stream).ok()?;
    serde_json::from_slice::<Value>(&body).ok()
}

pub(super) fn run_command(command: &str) -> Result<(), String> {
    let Some(reply) = request_json(SWAY_IPC_RUN_COMMAND, command) else {
        return Err("failed to run sway ipc command".to_string());
    };
    let Some(items) = reply.as_array() else {
        return Err("unexpected sway run_command response".to_string());
    };
    if items
        .iter()
        .all(|v| v.get("success").and_then(Value::as_bool) == Some(true))
    {
        Ok(())
    } else {
        Err(format!("sway command failed: {}", command))
    }
}

pub(super) fn output_name_by_index(index: usize) -> Option<String> {
    let outputs = request_json(SWAY_IPC_GET_OUTPUTS, "")?;
    let mut names: Vec<String> = outputs
        .as_array()?
        .iter()
        .filter(|o| o.get("active").and_then(Value::as_bool) == Some(true))
        .filter_map(|o| o.get("name").and_then(Value::as_str))
        .map(|s| s.to_string())
        .collect();
    names.sort();
    names.get(index).cloned()
}

pub(super) fn read_message(stream: &mut UnixStream) -> Result<(u32, Vec<u8>), ()> {
    let mut header = [0_u8; 14];
    stream.read_exact(&mut header).map_err(|_| ())?;
    if &header[..6] != SWAY_IPC_MAGIC {
        return Err(());
    }
    let len = u32::from_le_bytes([header[6], header[7], header[8], header[9]]) as usize;
    let msg_type = u32::from_le_bytes([header[10], header[11], header[12], header[13]]);
    let mut payload = vec![0_u8; len];
    stream.read_exact(&mut payload).map_err(|_| ())?;
    Ok((msg_type, payload))
}

fn write_message(stream: &mut UnixStream, msg_type: u32, payload: &[u8]) -> Result<(), ()> {
    let len = payload.len() as u32;
    stream.write_all(SWAY_IPC_MAGIC).map_err(|_| ())?;
    stream.write_all(&len.to_le_bytes()).map_err(|_| ())?;
    stream.write_all(&msg_type.to_le_bytes()).map_err(|_| ())?;
    stream.write_all(payload).map_err(|_| ())?;
    Ok(())
}
