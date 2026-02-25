package.preload["lush.scheduler"] = function()
  local scheduler = {}

  function scheduler.every(seconds, callback)
    return _lush_scheduler_every(math.max(1, math.floor((seconds or 1) * 1000)), callback)
  end

  function scheduler.after(seconds, callback)
    return _lush_scheduler_after(math.max(1, math.floor((seconds or 1) * 1000)), callback)
  end

  function scheduler.cancel(id)
    _lush_scheduler_cancel(id)
  end

  return scheduler
end
