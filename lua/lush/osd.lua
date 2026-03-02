package.preload["lush.osd"] = function()
  local osd = {}
  
  local hide_timers = {}

  local function get_scheduler()
    return require("lush.scheduler")
  end

  local function get_windows()
    return require("lush.windows")
  end

  local function get_signal()
    return require("lush.signal")
  end

  function osd.bind(opts)
    opts = opts or {}
    local name = opts.name
    if type(name) ~= "string" or name == "" then
      error("lush.osd.bind requires opts.name")
    end
    local signals = opts.signals or opts.show_on
    if type(signals) == "string" then
      signals = { signals }
    end
    if type(signals) ~= "table" or #signals == 0 then
      error("lush.osd.bind requires opts.signals (array)")
    end
    local timeout = opts.timeout or opts.timeout_ms or 1200
    local timeout_secs = math.max(1, math.floor(timeout)) / 1000.0

    local signal = get_signal()
    local windows = get_windows()
    local scheduler = get_scheduler()

    for _, signal_name in ipairs(signals) do
      signal.on(signal_name, function()
        windows.set_visible(name, true)
        
        if hide_timers[name] ~= nil then
          scheduler.cancel(hide_timers[name])
          hide_timers[name] = nil
        end
        
        hide_timers[name] = scheduler.after(timeout_secs, function()
          windows.set_visible(name, false)
          hide_timers[name] = nil
        end)
      end, { immediate = false })
    end
  end

  function osd.create(opts)
    opts = opts or {}
    local window = opts.window
    if type(window) ~= "table" then
      error("lush.osd.create requires opts.window (ui.window table)")
    end
    local name = opts.name or window.name
    if type(name) ~= "string" or name == "" then
      error("lush.osd.create requires a named window (opts.name or window.name)")
    end
    osd.bind({
      name = name,
      signals = opts.signals or opts.show_on,
      timeout = opts.timeout or opts.timeout_ms,
    })
    return {
      name = name,
      window = window,
    }
  end

  return osd
end
