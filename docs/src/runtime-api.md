# Runtime API

`local lush = require("lush")`

top-level modules:

- `lush.ui`
- `lush.data`
- `lush.state`
- `lush.signal`
- `lush.windows`
- `lush.notifications`
- `lush.osd`
- `lush.scheduler`
- `lush.process`
- `lush.audio`
- `lush.store`

## `lush.data`

start providers explicitly:

```lua
lush.data.use("cpu", { interval = 2 })
lush.data.use("network", { interval = 1, iface = "wlan0" })
lush.data.use("disk", { path = "/" })
lush.data.use("audio")
lush.data.use("compositor", { output = "focused" })
```

stop a provider if you have to:

```lua
lush.data.unuse("cpu")
```

reactive watch:

```lua
local stop = lush.data.watch("cpu", { immediate = true }, function(snapshot)
  print(snapshot["data.cpu.percent"])
end)

-- if you want to stop:
stop()
```

provider signal keys are listed in [signals reference](signals.md).

## `lush.state` and `lush.signal`

both write to the same runtime signal bus.

```lua
lush.state.set("app.mode", "normal")
lush.signal.emit("app.mode", "normal")
```

watch one key:

```lua
local unwatch = lush.state.watch("app.mode", function(v)
  print("mode: ", v)
end, { immediate = true })
```

watch many:

```lua
local unwatch = lush.state.watch_many({
  "data.cpu.percent",
  "data.memory.percent",
}, function(values)
  print(values["data.cpu.percent"], values["data.memory.percent"])
end)
```

## `lush.windows`

control named windows:

```lua
lush.windows.toggle("bar")
lush.windows.open("bar")
lush.windows.close("bar")
lush.windows.set_visible("control-center", true)
```

## `lush.process`

watch shell command output:

```lua
local stop = lush.process.watch("date +%H:%M:%S", 1, function(out)
  lush.state.set("custom.clock", out)
end, {
  queue_policy = "latest", -- or "drop"
})
```

## `lush.scheduler`

```lua
local id = lush.scheduler.after(1.2, function()
  print("once")
end)

local every_id = lush.scheduler.every(5, function()
  print("tick")
end)

lush.scheduler.cancel(every_id)
```

## `lush.osd`

bind a named window to one or more signals and auto-hide:

```lua
lush.osd.create({
  window = volume_osd, -- ui.window table with name
  signals = { "data.audio.volume", "data.audio.muted" },
  timeout = 1200,
})
```

use this instead of manual `state.watch + scheduler` show/hide loops.
