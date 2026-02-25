# usage

## config location

lush loads the first existing file from:

1. `LUSH_CONFIG`
2. `$XDG_CONFIG_HOME/lush/config.lua`
3. `$HOME/.config/lush/config.lua`
4. `./config.lua`

## minimal config

```lua
local lush = require("lush")
local ui = lush.ui

ui.css("style.css")

ui.windows({
  ui.window({
    name = "bar",
    position = "top",
    layer = "top",
    exclusive = true,
    height = 30,
    root = ui.hbox({
      spacing = 8,
      children = {
        ui.label({ text = "hello" }),
        ui.clock({ format = "%H:%M", interval = 60 }),
      },
    }),
  }),
})
```

## common patterns

### daemon control

start lush daemon:

```bash
lush daemon
```

control it from another shell:

```bash
lush ping
lush list
lush toggle bar
lush reload
lush reload-css
```

### use data providers

```lua
local lush = require("lush")

lush.data.use("cpu", { interval = 2 })
lush.data.use("memory", { interval = 3 })
lush.data.use("network", { interval = 1, iface = "wlan0" }) -- optional iface
lush.data.use("disk", { interval = 10, path = "/" }) -- optional path
lush.data.use("battery")

-- event driven with fallback refresh
lush.data.use("bluetooth", { interval = 120 })
lush.data.use("mpris", { interval = 20 })

lush.data.use("compositor", { output = "focused" }) -- optional output
```

you can then bind widgets directly to emitted keys

see [signals](signals.md) for reference

### watch a command

```lua
local lush = require("lush")

local id = lush.process.watch("sleep 2 && echo tick", 1, function(output)
  lush.state.set("watch.tick", output)
end, {
  queue_policy = "latest", -- default: keep at most one pending rerun
  -- queue_policy = "drop", -- skip ticks while command is still running
})
```

### bind state to labels

```lua
local lush = require("lush")
local ui = lush.ui

lush.data.use("battery")
lush.data.use("cpu", { interval = 2 })

local label = ui.label({
  bind = "data.cpu.percent",
  format = "cpu {value}%",
  format_states = {
    critical = "cpu {value} !",
    default = "cpu {value}",
  },
})

local battery = ui.label({
  binds = {
    ["battery.percent"] = "data.battery.percent",
    ["battery.state"] = "data.battery.state",
  },
  format = "bat {battery.percent}% ({battery.state})",
})

local battery_state = ui.label({
  binds = {
    ["battery.state"] = "data.battery.state",
    ["battery.percent"] = "data.battery.percent",
  },
  format = "bat [{battery.state}] {battery.percent}%",
  rules = {
    { target = "literal", match = "bat", color = "#928374", weight = "bold" },
    { target = "value", token = "battery.state", match = "charging", color = "#8ec07c", weight = "bold" },
    { target = "value", token = "battery.state", match = "dis*", color = "#d79921", weight = "bold" },
    { target = "value", token = "battery.percent", format = "{value}%" },
    { target = "value", token = "battery.percent", max = 20, color = "#fb4934", weight = "bold" },
    { target = "value", token = "battery.percent", min = 21, max = 40, color = "#fabd2f", weight = "bold" },
    { target = "value", token = "battery.percent", min = 41, color = "#8ec07c" },
  },
})
```

label basics:
- without `rules`, `ui.label` just renders the resolved format text.
- placeholders (`{value}`, `{state}`, `{token}`) are matched as `target = "value"`.
- plain text in `format` (like `bat`, `%`, punctuation) is matched as `target = "literal"`.
- rules are merged in order; put defaults first and overrides later.
- use `match`/`token` globs (`*`, `?`) for string matching and `min`/`max` for thresholds.
- you can set `class` in a rule to add css class(es) when that rule matches.

### formatting

`label`, `button`, and `workspaces` support `format` and `format_states`.
`clock` keeps `format` for strftime and uses `display_format` for placeholders.

```lua
ui.clock({
  format = "%H:%M:%S",
  display_format = "[{value}]",
  class_bind = "clock.mode",
  format_states = {
    default = "{value}",
    alt = "<{time}>",
  },
})

lush.process.watch("echo ok", 2, function(output)
  lush.state.set("shell.out", output)
end)

ui.label({
  bind = "shell.out",
  format = "shell {value}",
  class_bind = "shell.mode",
  format_states = {
    warn = "shell ! {value}",
  },
})
```

### toggle windows from Lua

```lua
lush.windows.toggle("panel")
lush.windows.set_visible("popup", true)
```
