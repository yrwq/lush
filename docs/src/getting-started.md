# Getting Started

## installation

runtime requirements:

- `gtk4`
- `gtk4-layer-shell`
- `libpulse`
- `libudev`

install from crates:

```bash
cargo install lushell
```

## create config

`~/.config/lush/init.lua`

```lua
local lush = require("lush")
local ui = lush.ui

ui.css("style.css")

lush.data.use("cpu", { interval = 2 })
lush.data.use("memory", { interval = 2 })

ui.windows({
  ui.window({
    name = "bar",
    position = "top",
    exclusive = true,
    height = 30,
    root = ui.hbox({
      spacing = 8,
      children = {
        ui.label({
          bind = "data.cpu.percent",
          format = "cpu {value}%",
        }),
        ui.label({
          bind = "data.memory.percent",
          format = "mem {value}%",
        }),
        ui.clock({
          format = "%H:%M:%S",
        }),
      },
    }),
  }),
})
```

will use your gtk theme by default if it supports gtk4

`~/.config/lush/style.css`

```css
* {
  font-family: monospace;
  font-size: 14px;
}
```

## run

with default config location:

```bash
lush
```

## control from shell

```bash
lush ping
lush list
lush toggle bar
lush reload
lush reload-css
```

## config lookup order

1. `LUSH_CONFIG`
2. `$XDG_CONFIG_HOME/lush/init.lua`
3. `$HOME/.config/lush/init.lua`
4. `./init.lua`

if `LUSH_CONFIG` is a directory, lush uses `<dir>/init.lua`.

## next

- learn runtime modules in [runutime api](runtime-api.md)
- see all widget fields in [widgets reference](widgets.md)
- see all emitted keys in [signals reference](signals.md)
- copy ready patterns from [recipes](recipes.md)
