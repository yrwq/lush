package.preload["lush.signal"] = function()
  local events = require("lush.events")
  local state = require("lush.state")
  local signal = {}

  function signal.on(name, callback, opts)
    if name == "*" then
      local unsubscribe = events.on_any(callback)
      if opts and opts.immediate then
        local snapshot = state.snapshot()
        for signal_name, signal_value in pairs(snapshot) do
          callback(signal_name, signal_value)
        end
      end
      return unsubscribe
    end
    return state.watch(name, callback, opts)
  end

  function signal.emit(name, value)
    state.set(name, value or "")
  end

  return signal
end
