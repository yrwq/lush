use std::collections::HashMap;
use std::time::Duration;

use zbus::blocking::fdo::DBusProxy as BlockingDBusProxy;
use zbus::blocking::{Connection as BlockingConnection, Proxy as BlockingProxy};
use zbus::export::futures_util::StreamExt;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, MessageType};

use super::evented::EventedProvider;
use crate::runtime::signal_bus::SignalBus;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";
const ROOT_IFACE: &str = "org.mpris.MediaPlayer2";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MprisSnapshot {
    available: bool,
    player: String,
    status: String,
    title: String,
    artist: String,
    album: String,
    art_url: String,
    length_us: String,
    position_us: String,
    summary: String,
}

#[derive(Clone, Debug, Default)]
struct PlayerSnapshot {
    service: String,
    identity: String,
    status: String,
    title: String,
    artist: String,
    album: String,
    art_url: String,
    length_us: String,
    position_us: String,
}

pub struct MprisWatcher {
    _evented: EventedProvider,
    signal_tasks: Vec<glib::JoinHandle<()>>,
}

impl Drop for MprisWatcher {
    fn drop(&mut self) {
        for task in self.signal_tasks.drain(..) {
            task.abort();
        }
    }
}

pub fn start_evented(bus: SignalBus, interval_seconds: u32) -> MprisWatcher {
    let fallback_interval = interval_seconds.max(1);
    let (snapshot_tx, snapshot_rx) = async_channel::unbounded::<MprisSnapshot>();
    let bus_for_snapshots = bus.clone();
    super::evented::spawn_snapshot_drain(snapshot_rx, move |snapshot| {
        apply_snapshot(&bus_for_snapshots, &snapshot);
    });

    let (trigger_tx, trigger_rx) = super::evented::trigger_channel();
    let mut evented = EventedProvider::new();
    let stop = evented.stop_flag();
    let collector_thread = super::evented::spawn_periodic_collector(
        "data.mpris",
        stop,
        trigger_rx,
        Duration::from_secs(fallback_interval as u64),
        snapshot_tx,
        move |_reason| collect_snapshot(),
    );
    evented.push_thread(collector_thread);

    let signal_tasks = vec![
        spawn_signal_listener(build_player_properties_rule(), trigger_tx.clone()),
        spawn_signal_listener(build_name_owner_rule(), trigger_tx.clone()),
    ];

    MprisWatcher {
        _evented: evented,
        signal_tasks,
    }
}

fn spawn_signal_listener(
    rule: Result<MatchRule<'static>, zbus::Error>,
    trigger_tx: std::sync::mpsc::SyncSender<()>,
) -> glib::JoinHandle<()> {
    glib::MainContext::default().spawn_local(async move {
        let Ok(rule) = rule else {
            log::debug!("data.mpris: failed to build dbus match rule");
            return;
        };
        let conn = match Connection::session().await {
            Ok(conn) => conn,
            Err(err) => {
                log::debug!("data.mpris: failed to connect to session dbus: {}", err);
                return;
            }
        };
        let mut stream = match zbus::MessageStream::for_match_rule(rule, &conn, Some(64)).await {
            Ok(stream) => stream,
            Err(err) => {
                log::debug!("data.mpris: failed to subscribe to dbus signals: {}", err);
                return;
            }
        };
        while let Some(msg) = stream.next().await {
            if msg.is_ok() {
                super::evented::trigger_now(&trigger_tx);
            }
        }
    })
}

fn build_player_properties_rule() -> Result<MatchRule<'static>, zbus::Error> {
    let builder = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path(MPRIS_PATH)?;
    Ok(builder.add_arg(PLAYER_IFACE)?.build().to_owned())
}

fn build_name_owner_rule() -> Result<MatchRule<'static>, zbus::Error> {
    let builder = MatchRule::builder()
        .msg_type(MessageType::Signal)
        .sender("org.freedesktop.DBus")?
        .interface("org.freedesktop.DBus")?
        .member("NameOwnerChanged")?;
    Ok(builder.arg0ns("org.mpris.MediaPlayer2")?.build().to_owned())
}

fn collect_snapshot() -> Result<MprisSnapshot, String> {
    let conn = BlockingConnection::session().map_err(|e| format!("dbus connect: {}", e))?;
    let dbus = BlockingDBusProxy::new(&conn).map_err(|e| format!("dbus proxy: {}", e))?;
    let names = dbus
        .list_names()
        .map_err(|e| format!("ListNames failed: {}", e))?;

    let mut players = Vec::new();
    for name in names {
        let service = name.as_str();
        if !service.starts_with(MPRIS_PREFIX) {
            continue;
        }
        if let Some(player) = collect_player(&conn, service) {
            players.push(player);
        }
    }

    if players.is_empty() {
        return Ok(MprisSnapshot {
            available: false,
            status: "stopped".to_string(),
            summary: "no media".to_string(),
            ..MprisSnapshot::default()
        });
    }

    players.sort_by(compare_players);
    let picked = players.remove(0);
    Ok(MprisSnapshot {
        available: true,
        player: picked.identity.clone(),
        status: picked.status.clone(),
        title: picked.title.clone(),
        artist: picked.artist.clone(),
        album: picked.album.clone(),
        art_url: picked.art_url.clone(),
        length_us: picked.length_us.clone(),
        position_us: picked.position_us.clone(),
        summary: build_summary(&picked),
    })
}

fn collect_player(conn: &BlockingConnection, service: &str) -> Option<PlayerSnapshot> {
    let player_proxy = BlockingProxy::new(conn, service, MPRIS_PATH, PLAYER_IFACE).ok()?;
    let root_proxy = BlockingProxy::new(conn, service, MPRIS_PATH, ROOT_IFACE).ok();

    let status = player_proxy
        .get_property::<String>("PlaybackStatus")
        .ok()
        .map(|v| normalize_ws(&v).to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());
    let metadata = player_proxy
        .get_property::<HashMap<String, OwnedValue>>("Metadata")
        .ok()
        .unwrap_or_default();
    let position_us = player_proxy
        .get_property::<i64>("Position")
        .ok()
        .map(|v| v.max(0).to_string())
        .unwrap_or_default();
    let identity = root_proxy
        .as_ref()
        .and_then(|proxy| proxy.get_property::<String>("Identity").ok())
        .map(|v| normalize_ws(&v))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| service.trim_start_matches(MPRIS_PREFIX).to_string());

    Some(PlayerSnapshot {
        service: service.to_string(),
        identity,
        status,
        title: metadata_title(&metadata),
        artist: metadata_artist(&metadata),
        album: metadata_string(&metadata, "xesam:album"),
        art_url: metadata_string(&metadata, "mpris:artUrl"),
        length_us: metadata_i64(&metadata, "mpris:length")
            .map(|v| v.max(0).to_string())
            .unwrap_or_default(),
        position_us,
    })
}

fn compare_players(a: &PlayerSnapshot, b: &PlayerSnapshot) -> std::cmp::Ordering {
    player_rank(&a.status)
        .cmp(&player_rank(&b.status))
        .then_with(|| a.service.cmp(&b.service))
}

fn player_rank(status: &str) -> u8 {
    match status {
        "playing" => 0,
        "paused" => 1,
        "stopped" => 2,
        _ => 3,
    }
}

fn metadata_title(metadata: &HashMap<String, OwnedValue>) -> String {
    metadata_string(metadata, "xesam:title")
}

fn metadata_artist(metadata: &HashMap<String, OwnedValue>) -> String {
    let artist = metadata_string(metadata, "xesam:artist");
    if !artist.is_empty() {
        return artist;
    }
    let Some(value) = metadata
        .get("xesam:artist")
        .and_then(|v| v.try_clone().ok())
    else {
        return String::new();
    };
    if let Ok(values) = Vec::<String>::try_from(value) {
        return values
            .into_iter()
            .map(|v| normalize_ws(&v))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }
    String::new()
}

fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> String {
    let Some(raw) = metadata.get(key) else {
        return String::new();
    };
    if let Some(value) = raw.try_clone().ok()
        && let Ok(v) = String::try_from(value)
    {
        return normalize_ws(&v);
    }
    if let Some(value) = raw.try_clone().ok()
        && let Ok(v) = Vec::<String>::try_from(value)
    {
        return v
            .into_iter()
            .map(|item| normalize_ws(&item))
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }
    String::new()
}

fn metadata_i64(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    let raw = metadata.get(key)?;
    if let Some(value) = raw.try_clone().ok()
        && let Ok(v) = i64::try_from(value)
    {
        return Some(v);
    }
    if let Some(value) = raw.try_clone().ok()
        && let Ok(v) = i32::try_from(value)
    {
        return Some(v as i64);
    }
    if let Some(value) = raw.try_clone().ok()
        && let Ok(v) = u64::try_from(value)
    {
        return Some(v.min(i64::MAX as u64) as i64);
    }
    None
}

fn build_summary(player: &PlayerSnapshot) -> String {
    if player.title.is_empty() && player.artist.is_empty() {
        return format!("{} ({})", player.identity, player.status);
    }
    if player.artist.is_empty() {
        return format!("{} ({})", player.title, player.status);
    }
    format!("{} - {} ({})", player.artist, player.title, player.status)
}

fn normalize_ws(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn apply_snapshot(bus: &SignalBus, snapshot: &MprisSnapshot) {
    bus.batch(|| {
        bus.set(
            "data.mpris.available",
            if snapshot.available { "1" } else { "0" },
        );
        bus.set("data.mpris.player", &snapshot.player);
        bus.set("data.mpris.status", &snapshot.status);
        bus.set("data.mpris.title", &snapshot.title);
        bus.set("data.mpris.artist", &snapshot.artist);
        bus.set("data.mpris.album", &snapshot.album);
        bus.set("data.mpris.art_url", &snapshot.art_url);
        bus.set("data.mpris.length_us", &snapshot.length_us);
        bus.set("data.mpris.position_us", &snapshot.position_us);
        bus.set("data.mpris.summary", &snapshot.summary);
    });
}
