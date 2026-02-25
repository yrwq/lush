package.preload["lush.ui"] = function()
  local ui = {}

  for name, ctor in pairs(_lush_widget_ctors or {}) do
    ui[name] = ctor
  end

  function ui.windows(list)
    _lush_ui_set_windows(list)
  end

  function ui.css(path)
    _lush_ui_set_css(path)
  end

  return ui
end
