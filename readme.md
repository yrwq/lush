

# 🌿 lush

> [!WARNING]
> lush is in a very early phase, expect breaking changes or incomplete docs

lightweight, extensible wayland shell and widget framework. build completely custom bars, panels, and desktop widgets using lua.

![preview](https://github.com/user-attachments/assets/1426008f-db10-4229-b508-0f88b395549f)


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

[docs](https://yrwq.github.io/lush/getting-started.html)

[examples](./examples) 

Install via `cargo install lushell` and run `lush daemon` to start.

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

## notes

if you find a bug feel free to open an issue or even better make a pull request <3

[contributing](docs/src/contributing.md)

---

*lush is inspired by projects like [AGS](https://github.com/Aylur/ags), [Eww](https://github.com/elkowar/eww) and [quickshell](https://github.com/quickshell-mirror/quickshell)*
