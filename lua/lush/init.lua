package.preload["lush.init"] = function()
  local lush = {
    state = require("lush.state"),
    signal = require("lush.signal"),
    windows = require("lush.windows"),
    notifications = require("lush.notifications"),
    scheduler = require("lush.scheduler"),
    process = require("lush.process"),
    audio = require("lush.audio"),
    data = require("lush.data"),
    ui = require("lush.ui"),
    store = require("lush.store"),
  }

  return lush
end
