package.preload["lush.store"] = function()
  local state = require("lush.state")

  local function create_store(initial)
    state.update(initial)
    local store = {}

    function store:get(name, default)
      if name == nil then
        return state.snapshot()
      end
      return state.get(name, default)
    end

    function store:set(name, value)
      if type(name) == "table" then
        state.update(name)
        return
      end
      state.set(name, value)
    end

    function store:watch(name, callback, opts)
      return state.watch(name, callback, opts)
    end

    function store:watch_many(names, callback, opts)
      return state.watch_many(names, callback, opts)
    end

    return store
  end

  return create_store
end
