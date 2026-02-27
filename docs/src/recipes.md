# recipes

patterns you can copy into your config.

## volume osd popup

```lua
local lush = require("lush")
local ui = lush.ui

local volume_osd = ui.window({
  name = "volume-osd",
  visible = false,
  layer = "overlay",
  anchors = { "top", "left", "right" },
  margin_top = 48,
  root = ui.hbox({
    class = "volume-osd",
    spacing = 8,
    children = {
      ui.label({
        class_bind = "data.audio.muted",
        format_states = {
          ["1"] = "󰝟",
          ["0"] = "󰕾",
          default = "󰕾",
        },
      }),
      ui.progress({
        bind = "data.audio.volume",
        min = 0,
        max = 100,
      }),
      ui.label({
        binds = {
          ["v"] = "data.audio.volume",
          ["m"] = "data.audio.muted",
        },
        format_states = {
          ["1"] = "muted",
          ["0"] = "{v}%",
          default = "{v}%",
        },
      }),
    },
  }),
})

lush.osd.create({
  window = volume_osd,
  signals = {
    "data.audio.volume",
    "data.audio.muted",
  },
  timeout = 1200,
})
```

## focused window label

```lua
ui.label({
  bind = "data.compositor.focused_window.title",
  format = "{value}",
  max_chars = 40,
  ellipsize = "end",
})
```

## workspace clients as glyphs

```lua
ui.workspaces({
  output = "focused",
  count = 5,
  show_clients = true,
  clients_use_glyphs = true,
  clients_max_items = 4,
  clients_glyph_fallback = "",
  clients_rules = {
    { class = "*firefox*", icon = "󰈹" },
    { class = "foot", icon = "" },
  },
})
```

## hide widget by signal

```lua
ui.label({
  bind = "data.bluetooth.connected_name",
  visible_bind = "data.bluetooth.connected_count",
  format = "{value}",
})
```

## run external command and bind output

```lua
lush.process.watch("echo hello", 5, function(out)
  lush.state.set("custom.value", out)
end)

ui.label({
  bind = "custom.value",
  format = "{value}",
})
```
