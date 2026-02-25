package.preload["lush.windows"] = function()
  local windows = {}

  function windows.open(name)
    _lush_window_open(name)
  end

  function windows.close(name)
    _lush_window_close(name)
  end

  function windows.toggle(name)
    _lush_window_toggle(name)
  end

  function windows.show(name)
    _lush_window_set_visible(name, true)
  end

  function windows.hide(name)
    _lush_window_set_visible(name, false)
  end

  function windows.set_visible(name, visible)
    _lush_window_set_visible(name, visible and true or false)
  end

  function windows.is_visible(name)
    return _lush_window_is_visible(name)
  end

  function windows.list()
    return _lush_window_list()
  end

  return windows
end
