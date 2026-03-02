# widgets

## window

`lush.ui.window({...})`

- `name: string?` named windows are controllable from `lush.windows.*`
- `output: string|integer?` monitor selector (index/name/connector)
- `visible: boolean` default `true`
- `layer: "background"|"bottom"|"top"|"overlay"` default `top`
- `exclusive: boolean` reserve layer-shell zone
- `position: "top"|"bottom"|"left"|"right"` default `top` anchors
- `anchor: string|{string,...}` explicit anchors override `position`
- `width: integer?`
- `height: integer?`
- `margin_top|margin_bottom|margin_left|margin_right: integer` default `0`
- `root: widget` required

## common widget

supported by all widget kinds:

- `class: string`
- `classes: {string,...}`
- `visible: boolean`
- `visible_bind: string` signal key mapped to widget visibility
- `width: integer`
- `height: integer`
- `hexpand: boolean`
- `vexpand: boolean`
- `halign: "fill"|"start"|"center"|"end"`
- `valign: "fill"|"start"|"center"|"end"`
- `class_bind: string` state key to map to css class

state class example:

```lua
ui.label({
  text = "net",
  class_bind = "network.status",
})
```

if `network.status = "down"`, widget gets css class `state-down`.

visibility bind example:

```lua
ui.image({
  bind = "notification.slot1.icon",
  visible_bind = "notification.slot1.icon",
})
```

`visible_bind` value rules:
- false when signal is: empty string, `0`, `false`, `off`, `no`, `hidden`
- true when signal is: `1`, `true`, `on`, `yes`, `visible`, `show`
- any other non-empty value is treated as true

## containers

### `hbox`
- fields: `children`, `spacing`

### `vbox`
- fields: `children`, `spacing`

### `centerbox`
- fields: `children`, `spacing`
- only first 3 children are used: start/center/end.
- currently only horizontal!

### `scroll`
- fields: `children`, `h_policy`, `v_policy`, `overlay_scrolling`, `kinetic_scrolling`, `propagate_natural_width`, `propagate_natural_height`, `min_content_width`, `min_content_height`
- only first child is used as scroll content.
- defaults:
  - `h_policy = "automatic"`
  - `v_policy = "automatic"`
  - `overlay_scrolling = false`
- policy values for `h_policy`/`v_policy`:
  - `automatic`
  - `always`
  - `never`
  - `external`

### `overlay`
- fields: `children`
- `children[1]` is base content.
- `children[2..]` are layered on top as overlays.

### `revealer`
- fields: `children`, `reveal`, `reveal_bind`, `transition`, `duration`
- only first child is used as revealer content.
- defaults:
  - `reveal = true`
  - `transition = "slide-down"`
  - `duration = 250` (milliseconds)
- `reveal_bind` maps signal text to visible state using the same truthy/falsey rules as `visible_bind`.
- transition values:
  - `none`
  - `crossfade`
  - `slide-right`
  - `slide-left`
  - `slide-up`
  - `slide-down` (default)

### `popover`
- fields: `children`, `position`, `autohide`, `has_arrow`
- `children[1]` is trigger widget; `children[2]` is popover content.
- extra children are ignored.
- defaults:
  - `position = "bottom"`
  - `autohide = true`
  - `has_arrow = true`
- `position` values: `top`, `bottom`, `left`, `right`

## content widgets

### `label`
- fields: `text`, `bind`, `binds`, `format`, `format_states`, `rules`, `on_click`, `max_chars`, `ellipsize`
- `on_click`: shell string, Lua function, or button map table.
- button map keys: `left`, `middle`, `right`, `wheel_up`, `wheel_down`
- `ellipsize`: `start`, `middle`, `end`, `none`
- renders with Pango markup and supports rule-based styling and text transforms.
- placeholders: `{value}`, `{text}`, `{state}`, plus any key from `binds`
- default format:
  - with `bind`: `{value}`
  - without `bind`: `{text}`
- `format_states`: optional map by state value (usually from `class_bind`) and/or `default`
- basics:
  - the format string is split into placeholder values (inside `{...}`) and literal chunks (plain text like `bat`, `%`, `(`, `)`).
  - rules can target placeholder values (`target = "value"`), literals (`target = "literal"`), or both (`target = "any"`/unset).
  - glob matching is supported in `match` and `token`: `*` = any sequence, `?` = single character.
  - numeric filters (`min`/`max`) only apply when the matched text parses as a number.
- each entry in `rules` is checked in order; all matching rules are merged, and later rules override earlier fields.
- rule filters:
  - `target`: `value` (placeholder values), `literal` (plain words/symbols in format string), or `any` (default)
  - `token`: placeholder key glob filter (for `target = "value"`), supports `*` and `?`
  - `match`: value/literal glob filter, supports `*` and `?`
  - `min` / `max`: numeric filter for numeric values
- style keys on each rule:
  - `format`, `class`, `color`, `background`, `weight`, `style`, `underline`, `font`, `size`, `rise`, `alpha`, `strikethrough`
- `format` on a rule transforms matched text before styling (supports `{value}`).
- `class` on a rule applies CSS class(es) to matched text fragments (`class = "warn"` or `class = "warn pill"`).

minimal example:

```lua
ui.label({
  bind = "data.battery.percent",
  format = "bat {value}%",
  rules = {
    { target = "literal", match = "bat", class = "label-prefix", color = "#928374", weight = "bold" },
    { target = "value", token = "value", format = "{value}%" },
    { target = "value", token = "value", max = 20, color = "#fb4934", weight = "bold" },
  },
})
```

example:

```lua
ui.label({
  binds = {
    ["battery.percent"] = "data.battery.percent",
    ["battery.state"] = "data.battery.state",
  },
  format = "bat {battery.percent} ({battery.state})",
  rules = {
    { target = "literal", match = "bat", color = "#928374", weight = "bold" },
    { target = "value", token = "battery.state", match = "charging", color = "#8ec07c", weight = "bold" },
    { target = "value", token = "battery.state", match = "dis*", color = "#fabd2f", weight = "bold" },
    { target = "value", token = "battery.state", match = "full", color = "#b8bb26", weight = "bold" },
    { target = "value", token = "battery.percent", format = "{value}%" },
    { target = "value", token = "battery.percent", max = 20, color = "#fb4934", weight = "bold" },
    { target = "value", token = "battery.percent", min = 21, max = 40, color = "#fabd2f", weight = "bold" },
    { target = "value", token = "battery.percent", min = 41, color = "#8ec07c" },
  },
})
```

### `button`
- fields: `text`, `bind`, `format`, `format_states`, `on_click`, `angle`
- `on_click`: shell string, Lua function, or button map table.
- button map keys: `left`, `middle`, `right`, `wheel_up`, `wheel_down`
- placeholders: `{text}`, `{value}`, `{state}`
- default format: `{text}`
- `format_states`: optional map by state value (usually from `class_bind`) and/or `default`
- special shell actions:
  - `lush.notifications.delete:{index}`
  - `lush.notifications.clear`

### `clock`
- fields: `format`, `display_format`, `format_states`, `interval`, `bind`, `spacing`, `angle`
- defaults:
  - `format = "%a %d.%b %H:%M:%S"`
  - `interval = 1` second
- `format` is the chrono/strftime pattern used to generate raw clock value.
- placeholders in `display_format`/`format_states`: `{value}`, `{time}`, `{state}`
- default `display_format`: `{value}`
- `format_states`: optional map by state value (usually from `class_bind`) and/or `default`

### `image`
- fields: `path`, `bind`, `fit`, `can_shrink`, `on_click`
- `on_click`: shell string, Lua function, or button map table.
- button map keys: `left`, `middle`, `right`, `wheel_up`, `wheel_down`
- `fit`: `contain` (default), `cover`, `fill`, `scale-down`
- source resolution order:
  1. explicit file/URI
  2. icon theme lookup by name

### `progress`
- fields: `bind`, `value`, `min`, `max`, `inverted`
- defaults:
  - `min = 0`
  - `max = 100`
  - `value = min` (when `bind` has no signal value)
- `bind` should contain a numeric value.
- normalized fraction: `(value - min) / (max - min)`, clamped to `0..1`.
- `inverted`: fills opposite direction when true

### `slider`
- fields: `bind`, `input_bind`, `value`, `min`, `max`, `step`, `scroll_step`, `orientation`, `inverted`, `draw_value`, `digits`
- defaults:
  - `min = 0`
  - `max = 100`
  - `value = min` (when `bind` has no signal value)
  - `step = 1`
  - `scroll_step = step`
  - `orientation = "horizontal"`
  - `draw_value = false`
  - `digits = 0`
- reads from `bind`.
- writes user changes to `input_bind` when set, otherwise writes back to `bind`.
- mouse wheel changes slider value by `scroll_step` (up increases, down decreases).
- `digits` controls numeric precision for emitted string values (`0..6`).

### `entry`
- fields: `text`, `bind`, `input_bind`, `activate_bind`, `placeholder`, `max_chars`, `autofocus`
- defaults:
  - `text = ""`
  - `autofocus = false`
- reads displayed text from `bind` when present, otherwise uses `text`.
- writes user edits to `input_bind` when set, otherwise writes back to `bind`.
- pressing Enter writes the current text to `activate_bind` and increments `activate_bind.__user_seq`.
- `placeholder` sets GTK placeholder text.
- `max_chars` limits entered text length.
- `autofocus` grabs focus when the widget is mapped.

### `workspaces`
- fields: `count`, `active_only`, `all_outputs`, `output`, `orientation`, `spacing`, `format`, `format_states`, `labels`, `state_labels`, `format_icons`, `show_clients`, `clients_max_items`, `clients_icon_size`, `clients_rules`, `clients_use_glyphs`, `clients_glyph_fallback`, `clients_spacing`, `angle`
- `orientation`: `horizontal` (default) or `vertical`
- `count` clamped to `1..32`
- `active_only`: show only focused/occupied/urgent workspaces when true
- `all_outputs`: include every output when true; otherwise uses selected/focused output
- placeholders in `format`/`format_states`: `{id}`, `{label}`, `{icon}`, `{clients}`, `{state}`
- `format_states`: optional map by state (`focused`, `occupied`, `unfocused`, `urgent`, `default`) overriding `format`
- `labels`: base label array by tag index
- `state_labels`: optional map of state -> label array by tag index
- `format_icons`: optional icon map with fallback keys:
  - `{id}.{state}` (example: `1.focused`)
  - `{state}` (example: `occupied`)
  - `{id}` (example: `3`)
  - `default`
- client rendering:
  - `show_clients`: render per-workspace client widgets after the workspace label
  - `clients_max_items`: max clients per workspace (`1..16`, default `4`)
  - `clients_icon_size`: icon size for image mode (`8..64`, default `12`)
  - `clients_spacing`: spacing between rendered clients (`0..24`, default `2`)
  - `clients_use_glyphs`: when true, render rule icon values as glyph text; when false, render as icon/path sources
  - `clients_rules`: shared matching rules (`class`, `title`, `icon`, `text`) with `*`/`?` globs and last-match-wins behavior
  - `clients_glyph_fallback`: fallback glyph when no rule icon matches in glyph mode
- styling hooks:
  - `.workspace-clients` container for per-workspace clients
  - `.workspace-client-glyph` glyph client item
  - `.workspace-client-icon` icon client item
  - `.focused-client` added to the currently focused client item
- backend note:
  - per-workspace client mapping is currently implemented for sway; other compositors may not populate workspace membership.

### `dock`
- fields: `orientation`, `output`, `all_outputs`, `spacing`, `max_items`, `format`, `format_states`, `image_map`, `icon_size`, `on_click`, `angle`
- `orientation`: `horizontal` (default) or `vertical`
- `output`: optional output selector like workspaces
- `all_outputs`: when true, ignores `output` and uses compositor-global focus stream
- `max_items` clamped to `1..128`
- `image_map`: pattern map (`*` and `?`) -> image source
  - value with `/` is treated as file path
  - otherwise treated as icon-theme name
- default render is icon-only; text is shown only when `format`/`format_states` is set
- placeholders in `format`/`format_states`:
  - `{label}` default label (`{title}`)
  - `{title}` window title fallbacking to app id
  - `{class}` app class/app id
  - `{app_id}` compositor app id/class
  - `{identifier}` compositor identifier (when available)
  - `{index}` 1-based slot
  - `{state}` `focused` or `default`
- `format_states` can override template for `focused` and `default`
- `icon_size`: icon/image pixel size, clamped to `8..128` (default `16`)
- `on_click` accepts a string action or a table:
  - keys: `left`, `middle`, `right`, `wheel_up`, `wheel_down`
  - values: `none`, `activate`, `close`, `minimize`, `restore`
  - default behavior: `{ left = "activate" }`

### `tray`
- fields: `orientation`, `spacing`, `icon_size`, `max_items`, `show_passive`, `hide_when_empty`
- `orientation`: `horizontal` (default) or `vertical`
- `spacing`: gap between tray items (default `6`)
- `icon_size`: icon pixel size, clamped to `8..128` (default `16`)
- `max_items`: max displayed items, clamped to `1..256` (default `32`)
- `show_passive`: include passive items when true (default `true`)
- `hide_when_empty`: hides tray widget when it has no rendered items (default `true`)
- click behavior:
  - left click -> `Activate(x, y)`
  - middle click -> `SecondaryActivate(x, y)`
  - right click -> internal DBusMenu popover when available, else `ContextMenu(x, y)`
  - wheel -> `Scroll(delta, "vertical")`
- styling hooks:
  - `.tray-container`
  - `.tray-item`
  - `.tray-icon`
  - `.tray-active`, `.tray-passive`, `.tray-needsattention`

### `list`
- fields: `children`, `bind`, `count`, `spacing`, `orientation`
- `children[1]` is row template cloned for each item.
- template token expansion:
  - `{item}` -> e.g. `item.1` (or custom base + index)
  - `{slot}` -> alias of `{item}`
  - `{base}` -> e.g. `item.`
  - `{index}` -> row index (`1`, `2`, ...)
- defaults:
  - `bind = "item."`
  - `count = 1` (clamped `1..256`)
  - `orientation = vertical`
- use `bind = "notification.slot"` and `visible_bind = "{item}.visible"` to render notification slots.
