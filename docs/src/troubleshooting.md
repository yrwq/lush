# troubleshooting

## no windows appear

1. run with logs:

```bash
RUST_LOG=debug lush -c /path/to/init.lua
```

2. check config file is actually loaded.
3. verify your `ui.windows({...})` contains at least one window.

## workspace widget is empty/disabled

if logs show `compositor workspace backend unavailable`, lush could not detect a supported backend.

- sway: `SWAYSOCK` is set and reachable
- hyprland: `HYPRLAND_INSTANCE_SIGNATURE` exists and sockets are present

## focus/workspace looks wrong on multi-monitor

`ui.workspaces` has output filtering behavior:

- `all_outputs = true`: includes all outputs
- `output = "focused"`: follows currently focused output
- `output = "DP-1"` or numeric index: pins to one output

if your bar is on monitor A but keyboard focus moves to monitor B, `output = "focused"` will follow B.
