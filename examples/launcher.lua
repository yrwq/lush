local lush = require("lush")
local ui = lush.ui
local state = lush.state
local process = lush.process

ui.css("examples/launcher.css")

local launcher = "launcher"
local query = "launcher.query"
local submit = "launcher.query.submit"
local slots = 8

local function trim(value)
  local text = tostring(value or "")
  text = text:gsub("^%s+", "")
  text = text:gsub("%s+$", "")
  return text
end

local function lower(value)
  return trim(value):lower()
end

local function shell_quote(value)
  return "'" .. tostring(value or ""):gsub("'", "'\\''") .. "'"
end

local function lines(value)
  local out = {}
  for line in tostring(value or ""):gmatch("[^\r\n]+") do
    line = trim(line)
    if line ~= "" then
      table.insert(out, line)
    end
  end
  return out
end

local function basename(path)
  return tostring(path or ""):match("([^/]+)$") or tostring(path or "")
end

local function key(slot, field)
  return string.format("launcher.item%d.%s", slot, field)
end

local function command_exists(name)
  local out = process.capture("command -v " .. shell_quote(name) .. " >/dev/null 2>&1 && printf 1 || printf 0")
  return trim(out) == "1"
end

local use_gtk_launch = command_exists("gtk-launch")
local home = os.getenv("HOME") or ""

local desktop_dirs = {
  home .. "/.local/share/applications",
  "/usr/local/share/applications",
  "/usr/share/applications",
  "/var/lib/flatpak/exports/share/applications",
  "/var/lib/snapd/desktop/applications",
}

local function sanitize_exec(value)
  local exec = trim(value)
  exec = exec:gsub("%%%%", "%%")
  exec = exec:gsub("%%[%a]", "")
  exec = exec:gsub("%s+", " ")
  return trim(exec)
end

local function parse_bool(value)
  value = lower(value)
  return value == "1" or value == "true" or value == "yes"
end

local function parse_desktop_file(path)
  local file = io.open(path, "r")
  if file == nil then
    return nil
  end

  local in_entry = false
  local app = {
    id = basename(path),
  }

  for line in file:lines() do
    if line == "[Desktop Entry]" then
      in_entry = true
    elseif in_entry and line:match("^%[.+%]$") then
      break
    elseif in_entry then
      local field, value = line:match("^([%w%-]+)%s*=%s*(.*)$")
      if field == "Name" then
        app.name = trim(value)
      elseif field == "GenericName" then
        app.generic = trim(value)
      elseif field == "Icon" then
        app.icon = trim(value)
      elseif field == "Exec" then
        app.exec = sanitize_exec(value)
      elseif field == "NoDisplay" then
        app.no_display = parse_bool(value)
      elseif field == "Hidden" then
        app.hidden = parse_bool(value)
      end
    end
  end

  file:close()

  if not in_entry or app.hidden or app.no_display then
    return nil
  end

  app.name = trim(app.name)
  app.generic = trim(app.generic)
  app.icon = trim(app.icon)
  app.exec = trim(app.exec)

  if app.name == "" then
    return nil
  end

  if use_gtk_launch then
    app.command = "gtk-launch " .. shell_quote(app.id)
  elseif app.exec ~= "" then
    app.command = app.exec
  else
    return nil
  end

  app.subtitle = app.generic ~= "" and app.generic or (app.exec ~= "" and app.exec or app.id)
  app.search = lower(table.concat({
    app.name,
    app.generic,
    app.id,
    app.exec,
  }, " "))

  return app
end

local function load_apps()
  local found = {}
  local seen = {}

  for _, dir in ipairs(desktop_dirs) do
    for _, path in ipairs(lines(process.capture(
      "[ -d " .. shell_quote(dir) .. " ] && find "
        .. shell_quote(dir)
        .. " -maxdepth 1 -type f -name '*.desktop' 2>/dev/null"
    ))) do
      if not seen[path] then
        seen[path] = true
        local app = parse_desktop_file(path)
        if app ~= nil then
          table.insert(found, app)
        end
      end
    end
  end

  table.sort(found, function(a, b)
    return a.name:lower() < b.name:lower()
  end)

  return found
end

local apps = load_apps()

local function rank(app, needle)
  if needle == "" then
    return 0
  end

  local name = app.name:lower()
  local id = app.id:lower()
  if name:find(needle, 1, true) == 1 then
    return 0
  end
  if id:find(needle, 1, true) == 1 then
    return 1
  end
  if name:find(needle, 1, true) ~= nil then
    return 2
  end
  if app.search:find(needle, 1, true) ~= nil then
    return 3
  end
  return nil
end

local function clear_results()
  local updates = {
    ["launcher.empty"] = #apps == 0 and "1" or "0",
  }

  for i = 1, slots do
    updates[key(i, "visible")] = "0"
    updates[key(i, "title")] = ""
    updates[key(i, "subtitle")] = ""
    updates[key(i, "icon")] = ""
    updates[key(i, "command")] = ""
  end

  state.update(updates)
end

local function refresh_results()
  local needle = lower(state.get(query, ""))
  local matches = {}

  for _, app in ipairs(apps) do
    local score = rank(app, needle)
    if score ~= nil then
      table.insert(matches, { score = score, app = app })
    end
  end

  table.sort(matches, function(a, b)
    if a.score ~= b.score then
      return a.score < b.score
    end
    return a.app.name:lower() < b.app.name:lower()
  end)

  local updates = {
    ["launcher.empty"] = (#matches == 0) and "1" or "0",
  }

  for i = 1, slots do
    local match = matches[i]
    updates[key(i, "visible")] = match ~= nil and "1" or "0"
    updates[key(i, "title")] = match ~= nil and match.app.name or ""
    updates[key(i, "subtitle")] = match ~= nil and match.app.subtitle or ""
    updates[key(i, "icon")] = match ~= nil and match.app.icon or ""
    updates[key(i, "command")] = match ~= nil and match.app.command or ""
  end

  state.update(updates)
end

local function launch(slot)
  local command = trim(state.get(key(slot, "command"), ""))
  if command == "" then
    return
  end

  process.spawn(command)
  state.set(query, "")
  lush.windows.close(launcher)
end

clear_results()
refresh_results()

state.watch(query, function()
  refresh_results()
end)

state.watch(submit .. ".__user_seq", function()
  launch(1)
end, { immediate = false })

local rows = {}
for i = 1, slots do
  table.insert(rows, ui.hbox({
    class = "launcher-row",
    visible_bind = key(i, "visible"),
    spacing = 14,
    children = {
      ui.image({
        class = "launcher-icon",
        bind = key(i, "icon"),
        visible_bind = key(i, "icon"),
        width = 20,
        height = 20,
      }),
      ui.vbox({
        class = "launcher-copy",
        spacing = 1,
        hexpand = true,
        children = {
          ui.button({
            class = "launcher-hit",
            bind = key(i, "title"),
            format = "{value}",
            hexpand = true,
            halign = "fill",
            on_click = function()
              launch(i)
            end,
          }),
          ui.label({
            class = "launcher-meta",
            bind = key(i, "subtitle"),
            max_chars = 72,
            ellipsize = "end",
            halign = "start",
          }),
        },
      }),
    },
  }))
end

ui.windows({
  ui.window({
    name = launcher,
    visible = true,
    layer = "overlay",
    exclusive = false,
    anchor = { "top", "bottom", "left", "right" },
    root = ui.vbox({
      class = "launcher-root",
      hexpand = true,
      vexpand = true,
      children = {
        ui.vbox({
          class = "launcher-panel",
          width = 760,
          spacing = 10,
          halign = "center",
          valign = "center",
          children = {
            ui.entry({
              class = "launcher-search",
              bind = query,
              input_bind = query,
              activate_bind = submit,
              placeholder = "Search apps",
              autofocus = true,
              hexpand = true,
            }),
            ui.label({
              class = "launcher-empty",
              text = "No applications found",
              visible_bind = "launcher.empty",
              halign = "start",
            }),
            ui.vbox({
              class = "launcher-results",
              spacing = 4,
              children = rows,
            }),
          },
        }),
      },
    }),
  }),
})
