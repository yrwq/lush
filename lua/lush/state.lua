package.preload["lush.state"] = function()
  local events = require("lush.events")
  local state = {}

  function state.get(name, default)
    local value = _lush_get(name)
    if value == nil then
      return default
    end
    return value
  end

  function state.set(name, value)
    _lush_set(name, tostring(value))
  end

  function state.update(values)
    for key, value in pairs(values or {}) do
      state.set(key, value)
    end
  end

  function state.snapshot()
    return _lush_snapshot()
  end

  function state.watch(name, callback, opts)
    local unsubscribe = events.on_key(name, callback)
    local immediate = opts == nil or opts.immediate ~= false
    if immediate then
      callback(state.get(name), name)
    end
    return unsubscribe
  end

  function state.watch_many(names, callback, opts)
    local unsubs = {}

    local function emit()
      local snapshot = state.snapshot()
      local picked = {}
      for _, key in ipairs(names or {}) do
        picked[key] = snapshot[key]
      end
      callback(picked, names)
    end

    for _, key in ipairs(names or {}) do
      table.insert(unsubs, state.watch(key, function()
        emit()
      end, { immediate = false }))
    end

    local immediate = opts == nil or opts.immediate ~= false
    if immediate then
      emit()
    end

    return function()
      for _, unsubscribe in ipairs(unsubs) do
        unsubscribe()
      end
    end
  end

  return state
end
