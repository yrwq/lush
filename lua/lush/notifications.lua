package.preload["lush.notifications"] = function()
  local notifications = {}

  function notifications.send(opts)
    opts = opts or {}
    _lush_notification_send(
      opts.title or "",
      opts.body or "",
      opts.icon or "",
      opts.urgency or "normal",
      opts.timeout or 5000
    )
  end

  function notifications.clear()
    _lush_notification_clear()
  end

  function notifications.delete(index)
    _lush_notification_delete(index)
  end

  return notifications
end
