local lush = require("lush")
local ui = lush.ui
local notif_center = require("examples.notifcenter")
local control_center = require("examples.controlcenter")
local music_popover = require("examples.musicpopover")

ui.css("examples/bar.css")

lush.data.use("memory", { interval = 1 })
lush.data.use("cpu", { interval = 1 })
lush.data.use("disk", { path = "/" })
lush.data.use("battery")
lush.data.use("audio")
lush.data.use("bluetooth")
lush.data.use("compositor")

local window_icon_rules = {
  { class = "*firefox*", icon = "󰈹", text = "Firefox" },
  { class = "thunar*", icon = "", text = "Files" },
  { title = "*YouTube*", icon = "󰗃", text = "YouTube" },
  { class = "foot", icon = "" },
  { title = "*Yazi*", icon = "", text = "Files" },
  { title = "*Discord*", icon = "", text = "Discord" },
}

lush.data.focused_window_iconify({
  fallback = "{icon} {title}",
  fallback_icon = "",
  rules = window_icon_rules,
})

local center = notif_center.create({
  lush = lush,
  ui = ui,
})

local controls = control_center.create({
  lush = lush,
  ui = ui,
  name = "control-center",
  width = 300,
})
local music = music_popover.create({
  lush = lush,
  ui = ui,
  interval = 1,
})

local bar_window = ui.window({
  height = 30,
  exclusive = true,
  name = "bar",
  root = ui.centerbox({
    class = "bar",
    spacing = 0,
    children = ({
      ui.hbox({
        spacing = 0,
        children = {
          ui.workspaces({
            class = "tags",
            all_outputs = false,
            active_only = false,
            count = 5,
            show_clients = true,
            clients_max_items = 4,
            clients_icon_size = 18,
            clients_spacing = 4,
            clients_rules = window_icon_rules,
            clients_use_glyphs = true,
            clients_glyph_fallback = "",
          }),
          -- ui.dock({
          --   class = "dock",
          --   max_items = 8,
          --   icon_size = 18,
          --   all_outputs = true,
          --   on_click = {
          --     left = "activate",
          --     middle = "minimize",
          --     right = "close",
          --     wheel_down = "minimize",
          --   },
          -- }),
          ui.label({
            format = " ",
          }),
          ui.hbox({
            class = "focused-window",
            spacing = 8,
            visible_bind = "data.compositor.focused_window.title",
            children = {
              ui.label({
                class = "focused-window-icon",
                bind = "data.compositor.focused_window.icon",
                format = "{value}",
                valign = "center",
              }),
              ui.label({
                class = "focused-window-title",
                bind = "data.compositor.focused_window.text",
                max_chars = 22,
                ellipsize = "end",
                format = "{value}",
                valign = "center",
              }),
            },
          }),
        },
      }),
      ui.hbox({
        spacing = 8,
        hexpand = true,
        halign = "center",
        children = {
          music.widget,
        },
      }),
      ui.hbox({
        spacing = 0,
        children = {
          ui.label({
            class = "prefix",
            format = "disk"
          }),
          ui.label({
            class = "wg",
            binds = {
              ["disk.used"] = "data.disk.used_gb",
              ["disk.total"] = "data.disk.total_gb",
            },
            format = "{disk.used}gb / {disk.total}gb"
          }),
          ui.label({
            class = "prefix",
            format = "cpu"
          }),
          ui.label({
            class = "wg",
            bind = "data.cpu.percent",
            format = "{value}%"
          }),
          ui.label({
            class = "prefix",
            format = "mem"
          }),
          ui.label({
            class = "wg",
            bind = "data.memory.percent",
            format = "{value}%"
          }),
          ui.label({
            class = "prefix",
            format = "vol"
          }),
          ui.label({
            class = "wg",
            class_bind = "data.audio.muted",
            binds = {
              ["audio.vol"] = "data.audio.volume",
              ["audio.muted"] = "data.audio.muted",
            },
            format = "{audio.vol}%",
            format_states = {
              ["1"] = "muted",
              ["0"] = "{audio.vol}%",
              default = "{audio.vol}%",
            },
          }),
          ui.label({
            class = "prefix",
            format = "bt",
            visible_bind = "data.bluetooth.connected_count",
          }),
          ui.label({
            classes = { "wg", "bt-value" },
            bind = "data.bluetooth.connected_name",
            class_bind = "data.bluetooth.state",
            visible_bind = "data.bluetooth.connected_count",
            format = "{value}",
            format_states = {
              unavailable = "n/a",
              off = "off",
              on = "on",
              connected = "{value}",
              default = "n/a",
            },
          }),
          ui.label({
            binds = {
              ["bat.percent"] = "data.battery.percent",
              ["bat.state"] = "data.battery.state"
            },
            format = "bat {bat.state}{bat.percent}",
            classes = { "wg", "bat-value" },
            rules = {
              { target = "literal", match = "bat", class = "prefix" },
              { target = "value", token = "bat.percent", format = "{value}%" },
              { target = "value", token = "bat.percent", max = 20, color = "#fb4934", weight = "bold" },
              { target = "value", token = "bat.percent", min = 21, max = 50, color = "#dca561", weight = "bold" },
              { target = "value", token = "bat.percent", min = 51, color = "#a9b665" },
              { target = "value", token = "bat.state", color = "#928374" },
              { target = "value", token = "bat.state", match = "charging", format = "chg ", color = "#a9b665", weight = "bold" },
              { target = "value", token = "bat.state", match = "discharging", format = "" },
              { target = "value", token = "bat.state", match = "full", format = "full ", color = "#a9b665", weight = "bold" },
              { target = "value", token = "bat.state", match = "unavailable", format = "n/a", color = "#7c6f64" },
            }
          }),
          ui.overlay({
            class = "overlay-host",
            children = {
              controls.toggle_button,
            },
          }),
          ui.overlay({
            class = "overlay-host",
            children = {
              center.widgets.bell_button,
              ui.label({
                class = "overlay-dot",
                bind = "notification.history_count",
                format = "{value}",
                visible_bind = "notification.history_count",
                halign = "end",
                valign = "start",
              }),
            },
          }),
          ui.tray({
            class = "tray",
            orientation = "horizontal",
            spacing = 6,
            icon_size = 16,
            show_passive = true,
          })
        }
      }),
    }),
  })
})

local windows = { bar_window, controls.window }
for _, win in ipairs(center.windows) do
  table.insert(windows, win)
end

ui.windows(windows)
