# 🌿 lush

> [!WARNING]
> lush is in a very early phase, expect breaking changes or incomplete docs

lightweight, extensible wayland shell and widget framework. it allows you to build completely custom bars, panels, and desktop widgets using lua.

---

## features

- **lua powered**: define your entire ui in lua.
- **wayland native**: built for wayland using `layer-shell`, supporting bars, panels, and overlays.
- **reactive state**: shared signal and state bus with widget bindings. update your ui when data changes.
- **data providers**: realtime monitoring for cpu, memory, network, disk, battery, and a lot more
- **notifications**: builtin dbus notification daemon that can be styled and displayed via Lua.
- **ipc**: control the daemon from scripts or shortcuts to toggle, or reload components.
- **styling**: use css for styling, with hot-reloading support.

## quickstart

known requirements:

- gtk4
- gtk4-layer-shell
- libpulse
- libudev

```bash
# install via cargo
cargo install --path .
```

```bash
# start the daemon with default config location
lush

# start with a specific config
lush -c examples/bar.lua
```

```bash
# list windows
lush list
# toggle a window named 'bar'
lush toggle bar
# reload the whole config
lush reload
# hot reload css
lush reload-css
```

## minimal example

```lua
local lush = require("lush")
local ui = lush.ui

-- enable data providers you need
lush.data.use("cpu", { interval = 2 })
lush.data.use("battery")

ui.windows({
  ui.window({
    name = "top_bar",
    position = "top",
    exclusive = true,
    root = ui.hbox({
      spacing = 10,
      children = {
        ui.label({ 
            -- bind to providers
            bind = "data.cpu.percent", 
            format = "CPU: {value}%" 
        }),
        ui.clock({ format = "%H:%M:%S" }),
        ui.label({ 
            bind = "data.battery.percent", 
            format = "BAT: {value}%" 
        }),
      }
    })
  })
})
```

## roadmap

- [ ] auto config reloading (file watcher)
- [ ] systray

widgets:

- [ ] entry

data providers:

- [ ] gpu usage
- [ ] cpu, gpu temp

## notes

if you find a bug feel free to open an issue or even better make a pull request <3

---

*lush is inspired by projects like [AGS](https://github.com/Aylur/ags), [Eww](https://github.com/elkowar/eww) and [quickshell](https://github.com/quickshell-mirror/quickshell)*
