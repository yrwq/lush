package.preload["lush.osd"] = function()
  local osd = {}

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
    _lush_osd_bind(name, signals, math.max(1, math.floor(timeout)))
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
