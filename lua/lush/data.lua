package.preload["lush.data"] = function()
  local state = require("lush.state")

  local data = {}

  local defs = {
    cpu = {
      signals = {
        "data.cpu.percent",
        "data.cpu.state",
        "data.cpu.user",
        "data.cpu.system",
        "data.cpu.idle",
        "data.cpu.total",
      },
    },
    memory = {
      signals = {
        "data.memory.percent",
        "data.memory.state",
        "data.memory.total_mb",
        "data.memory.used_mb",
        "data.memory.available_mb",
        "data.memory.total_gb",
        "data.memory.used_gb",
        "data.memory.available_gb",
      },
    },
    network = {
      signals = {
        "data.network.down_bps",
        "data.network.up_bps",
        "data.network.down_kibps",
        "data.network.up_kibps",
        "data.network.iface",
        "data.network.state",
        "data.network.down_total_bytes",
        "data.network.up_total_bytes",
        "data.network.ssid",
        "data.network.wifi_strength_percent",
        "data.network.wifi_signal_dbm",
      },
    },
    disk = {
      signals = {
        "data.disk.path",
        "data.disk.total_percent",
        "data.disk.used_percent",
        "data.disk.free_percent",
        "data.disk.total_gb",
        "data.disk.used_gb",
        "data.disk.free_gb",
        "data.disk.total_bytes",
        "data.disk.used_bytes",
        "data.disk.free_bytes",
      },
    },
    battery = {
      signals = {
        "data.battery.percent",
        "data.battery.state",
        "data.battery.time_left_min",
        "data.battery.power_w",
      },
    },
    audio = {
      signals = {
        "data.audio.volume",
        "data.audio.muted",
        "data.audio.sink",
      },
    },
    bluetooth = {
      signals = {
        "data.bluetooth.available",
        "data.bluetooth.powered",
        "data.bluetooth.connected_count",
        "data.bluetooth.connected_name",
        "data.bluetooth.connected_address",
        "data.bluetooth.connected_battery_percent",
        "data.bluetooth.adapter",
        "data.bluetooth.state",
        "data.bluetooth.summary",
      },
    },
    mpris = {
      signals = {
        "data.mpris.available",
        "data.mpris.player",
        "data.mpris.status",
        "data.mpris.title",
        "data.mpris.artist",
        "data.mpris.album",
        "data.mpris.art_url",
        "data.mpris.length_us",
        "data.mpris.position_us",
        "data.mpris.summary",
      },
    },
    compositor = {
      signals = {
        "data.compositor.name",
        "data.compositor.summary",
        "data.compositor.focused_mask",
        "data.compositor.occupied_mask",
        "data.compositor.urgent_mask",
        "data.compositor.focused_workspace",
        "data.compositor.focused_window.title",
        "data.compositor.focused_window.app_id",
        "data.compositor.focused_window.workspace",
        "data.compositor.focused_window.icon",
        "data.compositor.focused_window.text",
        "data.compositor.focused_window.display",
      },
    },
  }

  local function assert_provider(name)
    if defs[name] == nil then
      error("unknown data provider: " .. tostring(name))
    end
  end

  function data.list()
    return { "cpu", "memory", "network", "disk", "battery", "audio", "bluetooth", "mpris", "compositor" }
  end

  function data.use(name, opts)
    opts = opts or {}
    assert_provider(name)
    _lush_data_use(name, opts)
  end

  function data.unuse(name)
    assert_provider(name)
    _lush_data_unuse(name)
  end

  function data.snapshot(name)
    assert_provider(name)
    local out = {}
    for _, key in ipairs(defs[name].signals) do
      out[key] = state.get(key)
    end
    return out
  end

  function data.watch(name, opts, callback)
    if type(opts) == "function" and callback == nil then
      callback = opts
      opts = {}
    end
    opts = opts or {}
    assert_provider(name)
    if type(callback) ~= "function" then
      error("lush.data.watch requires a callback")
    end

    data.use(name, opts)

    local keys = defs[name].signals
    local unsubscribe = state.watch_many(keys, function(values)
      local picked = {}
      for _, key in ipairs(keys) do
        picked[key] = values[key]
      end
      callback(picked)
    end, { immediate = opts.immediate ~= false })

    return function()
      unsubscribe()
      data.unuse(name)
    end
  end

  local function glob_to_lua_pattern(glob)
    local escaped = tostring(glob or ""):gsub("([%%%^%$%(%)%.%[%]%+%-])", "%%%1")
    return "^" .. escaped:gsub("%*", ".*"):gsub("%?", ".") .. "$"
  end

  local function glob_match(value, glob)
    if glob == nil or glob == "" then
      return true
    end
    return tostring(value or ""):match(glob_to_lua_pattern(glob)) ~= nil
  end

  function data.focused_window_iconify(opts)
    opts = opts or {}
    local target_key = opts.target_key or "data.compositor.focused_window.display"
    local icon_key = opts.icon_key or "data.compositor.focused_window.icon"
    local text_key = opts.text_key or "data.compositor.focused_window.text"
    local class_key = opts.class_key or "data.compositor.focused_window.app_id"
    local title_key = opts.title_key or "data.compositor.focused_window.title"
    local fallback = opts.fallback or "{title}"
    local fallback_icon = opts.fallback_icon or ""
    local rules = opts.rules or {}

    local function resolve()
      local class_value = state.get(class_key, "")
      local title_value = state.get(title_key, "")
      local matched = nil

      for _, rule in ipairs(rules) do
        local ok_class = glob_match(class_value, rule.class)
        local ok_title = glob_match(title_value, rule.title)
        if ok_class and ok_title then
          matched = rule
        end
      end

      local icon = fallback_icon
      local text = title_value
      if matched then
        if matched.icon ~= nil then
          icon = tostring(matched.icon)
        end
        if matched.text ~= nil then
          text = tostring(matched.text)
        end
      end

      local display = fallback
        :gsub("{icon}", icon)
        :gsub("{title}", title_value)
        :gsub("{class}", class_value)
        :gsub("{text}", text)

      state.set(icon_key, icon)
      state.set(text_key, text)
      state.set(target_key, display)
    end

    local stop = state.watch_many({ class_key, title_key }, function()
      resolve()
    end, { immediate = true })

    return stop
  end

  return data
end
