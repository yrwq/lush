package.preload["lush.events"] = function()
  local scoped_watchers = {}
  local wildcard_watchers = {}
  local watcher_seq = 0

  local function next_id()
    watcher_seq = watcher_seq + 1
    return watcher_seq
  end

  local function add_watcher(bucket, callback)
    local id = next_id()
    bucket[id] = callback
    return function()
      bucket[id] = nil
    end
  end

  local function emit_bucket(bucket, ...)
    for _, callback in pairs(bucket) do
      local ok, err = pcall(callback, ...)
      if not ok then
        print("lush callback failed: " .. tostring(err))
      end
    end
  end

  local events = {}

  function events.on_key(name, callback)
    if scoped_watchers[name] == nil then
      scoped_watchers[name] = {}
    end
    return add_watcher(scoped_watchers[name], callback)
  end

  function events.on_any(callback)
    return add_watcher(wildcard_watchers, callback)
  end

  _G.__lush_dispatch = function(name, value)
    local scoped = scoped_watchers[name]
    if scoped then
      emit_bucket(scoped, value, name)
    end
    emit_bucket(wildcard_watchers, name, value)
  end

  return events
end
