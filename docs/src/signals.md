# signals

all keys are string-based values on the shared signal bus.

## window visibility keys

written by runtime when named windows are shown/hidden:

- `window.<name>.visible` -> `"1"` or `"0"`

## notification live keys

primary active notification mirror:

- `notification.visible`
- `notification.id`
- `notification.app_name`
- `notification.summary`
- `notification.title`
- `notification.body`
- `notification.icon`
- `notification.urgency`
- `notification.urgency_name`
- `notification.changed` (incremented once per logical notification update)
- `notification.active_count` (visible count in live stack)
- `notification.history_count` (visible count in history stack)

notification lifecycle event keys (last event snapshot):

- `notification.event` (`pushed` | `closed` | `history_cleared` | `history_deleted`)
- `notification.event_seq` (incremented per lifecycle event)
- `notification.event.id`
- `notification.event.app_name`
- `notification.event.summary`
- `notification.event.title`
- `notification.event.body`
- `notification.event.icon`
- `notification.event.urgency`
- `notification.event.urgency_name`
- `notification.event.history_index` (`0` unless `history_deleted`)

stack slots (`MAX_NOTIFICATION_SLOTS = 3`):

- `notification.slot1.*`
- `notification.slot2.*`
- `notification.slot3.*`

each slot has:

- `id`, `app_name`, `summary`, `title`, `body`, `icon`, `urgency`, `urgency_name`, `visible`

history slots (`MAX_HISTORY_SLOTS = 32`):

- `notification.history1.*` ... `notification.history32.*`

## notification control keys

observed by notification runtime:

- `notification.history_clear` or `notification.history.clear`
- `notification.history_delete` or `notification.history.delete`

trigger behavior:

- clear: clears all history
- delete: deletes one history row

## data providers (`lush.data`)

`cpu` provider:

- `data.cpu.percent`
- `data.cpu.state` (`normal` | `warn` | `critical`)
- `data.cpu.user`
- `data.cpu.system`
- `data.cpu.idle`
- `data.cpu.total`

`memory` provider:

- `data.memory.percent`
- `data.memory.state` (`normal` | `warn` | `critical`)
- `data.memory.total_mb`
- `data.memory.used_mb`
- `data.memory.available_mb`
- `data.memory.total_gb`
- `data.memory.used_gb`
- `data.memory.available_gb`

`network` provider:

- `data.network.down_bps`
- `data.network.up_bps`
- `data.network.down_kibps`
- `data.network.up_kibps`
- `data.network.iface` (`all` or requested interface name)
- `data.network.state` (`idle` | `active`)
- `data.network.down_total_bytes`
- `data.network.up_total_bytes`
- `data.network.ssid` (empty when unavailable/not Wi-Fi)
- `data.network.wifi_strength_percent` (`0..100`, `0` when unavailable)
- `data.network.wifi_signal_dbm` (empty when unavailable)

`disk` provider:

- `data.disk.path`
- `data.disk.total_percent` (always `100`)
- `data.disk.used_percent`
- `data.disk.free_percent`
- `data.disk.total_gb`
- `data.disk.used_gb`
- `data.disk.free_gb`
- `data.disk.total_bytes`
- `data.disk.used_bytes`
- `data.disk.free_bytes`

`battery` provider:

- `data.battery.percent`
- `data.battery.state` (`charging` | `discharging` | `full` | `unknown` | `unavailable`)
- `data.battery.time_left_min`
- `data.battery.power_w`

`audio` provider:

- `data.audio.volume` (`0..150`)
- `data.audio.muted` (`1` muted, `0` unmuted)
- `data.audio.sink`

`bluetooth` provider:

- `data.bluetooth.available` (`1` when adapter exists, else `0`)
- `data.bluetooth.powered` (`1` on, `0` off)
- `data.bluetooth.connected_count`
- `data.bluetooth.connected_name` (first connected device alias/name)
- `data.bluetooth.connected_address` (first connected device MAC)
- `data.bluetooth.connected_battery_percent` (`0..100` when bluez `Battery1` is available)
- `data.bluetooth.adapter` (adapter alias/name)
- `data.bluetooth.state` (`unavailable` | `off` | `on` | `connected`)
- `data.bluetooth.summary`

`mpris` provider:

- `data.mpris.available` (`1` when at least one MPRIS player exists, else `0`)
- `data.mpris.player` (selected player identity)
- `data.mpris.status` (`playing` | `paused` | `stopped` | `unknown`)
- `data.mpris.title`
- `data.mpris.artist`
- `data.mpris.album`
- `data.mpris.art_url` (cover art URI/path)
- `data.mpris.length_us` (track length in microseconds)
- `data.mpris.position_us` (playback position in microseconds)
- `data.mpris.summary`

`compositor` provider:

- `data.compositor.name`
- `data.compositor.summary`
- `data.compositor.focused_mask`
- `data.compositor.occupied_mask`
- `data.compositor.urgent_mask`
- `data.compositor.focused_workspace`
- `data.compositor.focused_window.title`
- `data.compositor.focused_window.app_id`
- `data.compositor.focused_window.workspace`

## state and signal note

`lush.state.set("x", "y")` and `lush.signal.emit("x", "y")` are equivalent, both write to the same runtime bus and dispatch watchers.
