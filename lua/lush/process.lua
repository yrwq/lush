package.preload["lush.process"] = function()
  local scheduler = require("lush.scheduler")
  local process = {}

  function process.capture(command)
    return _lush_process_capture(command)
  end

  function process.spawn(command)
    return _lush_process_spawn(command)
  end

  function process.watch(command, interval_seconds, callback, opts)
    opts = opts or {}
    local queue_policy = opts.queue_policy or "latest" -- "latest" | "drop"
    local in_flight = false
    local pending = false

    local function run()
      if in_flight then
        if queue_policy == "latest" then
          pending = true
        end
        return
      end

      in_flight = true
      _lush_process_capture_async(command, function(output)
        in_flight = false

        local ok, err = pcall(callback, output)
        if not ok then
          print("lush.process.watch callback failed: " .. tostring(err))
        end

        if pending then
          pending = false
          run()
        end
      end)
    end

    return scheduler.every(interval_seconds, run)
  end

  return process
end
