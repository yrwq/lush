local M = {}

function M.create(opts)
  opts = opts or {}
  local lush = opts.lush or require("lush")
  local ui = opts.ui or lush.ui
  local scheduler = lush.scheduler
  local process = lush.process

  local interval = opts.interval or 1
  local notify_on_change = opts.notify_on_change ~= false
  local notify_title = opts.notify_title or "Now Playing"
  local notify_icon = opts.notify_icon or "audio-x-generic"
  lush.data.use("mpris", { interval = interval })

  local seek_apply_timer = nil
  local seek_sync_timer = nil
  local suppress_seek_sync = false
  local suppress_seek_apply = false

  local function cancel_timer(timer_id)
    if timer_id ~= nil then
      scheduler.cancel(timer_id)
    end
    return nil
  end

  local function suppress_seek_sync_briefly(seconds)
    suppress_seek_sync = true
    seek_sync_timer = cancel_timer(seek_sync_timer)
    seek_sync_timer = scheduler.after(seconds, function()
      suppress_seek_sync = false
      seek_sync_timer = nil
    end)
  end

  local function parse_num(raw)
    local n = tonumber(raw or "")
    if n == nil then
      return 0
    end
    return n
  end

  local function fmt_mm_ss(seconds)
    local s = math.max(0, math.floor(seconds + 0.5))
    local m = math.floor(s / 60)
    local rem = s % 60
    return string.format("%d:%02d", m, rem)
  end

  local function sync_tick(step_seconds)
    local available = lush.state.get("music.available", "0") == "1"
    local status = lush.state.get("music.status", "stopped")
    if not available or status ~= "playing" then
      return
    end

    local duration_sec = parse_num(lush.state.get("music.duration_sec", "0"))
    local position_sec = parse_num(lush.state.get("music.position_sec", "0")) + step_seconds
    if duration_sec > 0 then
      position_sec = math.min(position_sec, duration_sec)
    end

    local seek_percent = 0
    if duration_sec > 0 then
      seek_percent = (position_sec / duration_sec) * 100
    end

    lush.state.set("music.position_sec", position_sec)
    lush.state.set("music.position", fmt_mm_ss(position_sec))
    if not suppress_seek_sync then
      suppress_seek_apply = true
      lush.state.set("music.seek_percent", string.format("%.2f", seek_percent))
    end
  end

  lush.state.watch_many({
    "data.mpris.available",
    "data.mpris.player",
    "data.mpris.status",
    "data.mpris.title",
    "data.mpris.artist",
    "data.mpris.art_url",
    "data.mpris.length_us",
    "data.mpris.position_us",
  }, function(values)
    local available = (values["data.mpris.available"] or "0") == "1"
    local status = values["data.mpris.status"] or "stopped"
    local title = values["data.mpris.title"] or ""
    local artist = values["data.mpris.artist"] or ""
    local player = values["data.mpris.player"] or "Media"
    local art_url = values["data.mpris.art_url"] or ""
    local length_us = parse_num(values["data.mpris.length_us"])
    local position_us = parse_num(values["data.mpris.position_us"])

    local duration_sec = math.max(0, math.floor((length_us / 1000000) + 0.5))
    local position_sec = math.max(0, math.floor((position_us / 1000000) + 0.5))
    if duration_sec > 0 then
      position_sec = math.min(position_sec, duration_sec)
    end

    local seek_percent = 0
    if duration_sec > 0 then
      seek_percent = (position_sec / duration_sec) * 100
    end

    if not suppress_seek_sync then
      suppress_seek_apply = true
      lush.state.set("music.seek_percent", string.format("%.2f", seek_percent))
    end

    lush.state.update({
      ["music.available"] = available and "1" or "0",
      ["music.status"] = status,
      ["music.duration_sec"] = duration_sec,
      ["music.position_sec"] = position_sec,
      ["music.title"] = title ~= "" and title or "No media",
      ["music.artist"] = artist ~= "" and artist or player,
      ["music.cover"] = art_url ~= "" and art_url or "audio-x-generic",
      ["music.position"] = fmt_mm_ss(position_sec),
      ["music.duration"] = fmt_mm_ss(duration_sec),
      ["music.play_icon"] = status == "playing" and "󰏤" or "󰐊",
    })
  end, { immediate = true })

  scheduler.every(1, function()
    sync_tick(1)
  end)

  lush.state.watch("music.seek_percent", function(value)
    if suppress_seek_apply then
      suppress_seek_apply = false
      return
    end
    if lush.state.get("music.available", "0") ~= "1" then
      return
    end

    local duration = parse_num(lush.state.get("data.mpris.length_us", "0")) / 1000000
    if duration <= 0 then
      return
    end

    local percent = parse_num(value)
    local clamped = math.max(0, math.min(100, percent))
    local target = math.floor((duration * clamped / 100) + 0.5)

    suppress_seek_sync_briefly(0.8)
    seek_apply_timer = cancel_timer(seek_apply_timer)
    seek_apply_timer = scheduler.after(0.09, function()
      process.spawn(string.format("playerctl position %d", target))
      seek_apply_timer = nil
    end)
  end, { immediate = false })

  if notify_on_change then
    local seeded = false
    local last_track_id = ""
    local pending_track_id = ""
    local notify_timer = nil

    local function schedule_track_notification(track_id, retry)
      notify_timer = cancel_timer(notify_timer)
      notify_timer = scheduler.after(0.2, function()
        if pending_track_id ~= track_id then
          notify_timer = nil
          return
        end

        local available = lush.state.get("data.mpris.available", "0") == "1"
        local player = lush.state.get("data.mpris.player", "")
        local title = lush.state.get("data.mpris.title", "")
        local artist = lush.state.get("data.mpris.artist", "")
        local art_url = lush.state.get("data.mpris.art_url", "")
        local current_track_id = string.format("%s|%s|%s", player, artist, title)

        if not available or title == "" or current_track_id ~= track_id then
          notify_timer = nil
          return
        end

        if art_url == "" and retry < 6 then
          schedule_track_notification(track_id, retry + 1)
          return
        end

        local body = title
        if artist ~= "" then
          body = string.format("%s - %s", artist, title)
        end

        lush.notifications.send({
          title = notify_title,
          body = body,
          icon = art_url ~= "" and art_url or notify_icon,
          urgency = "low",
          timeout = 2500,
        })

        last_track_id = track_id
        pending_track_id = ""
        notify_timer = nil
      end)
    end

    lush.state.watch_many({
      "data.mpris.available",
      "data.mpris.player",
      "data.mpris.title",
      "data.mpris.artist",
      "data.mpris.art_url",
    }, function(values)
      local available = (values["data.mpris.available"] or "0") == "1"
      local player = values["data.mpris.player"] or ""
      local title = values["data.mpris.title"] or ""
      local artist = values["data.mpris.artist"] or ""
      local art_url = values["data.mpris.art_url"] or ""
      local track_id = string.format("%s|%s|%s", player, artist, title)

      if not seeded then
        seeded = true
        last_track_id = track_id
        return
      end

      if not available or title == "" then
        return
      end

      if track_id == last_track_id then
        return
      end

      pending_track_id = track_id
      schedule_track_notification(track_id, 0)
    end, { immediate = true })
  end

  local widget = ui.popover({
    class = "clock-popover",
    position = "bottom",
    has_arrow = false,
    autohide = true,
    children = {
      ui.clock({
        classes = { "wg", "clock-trigger" },
        format = "%a %m.%d %I:%M %p",
        display_format = "{value}",
        interval = 60,
      }),
      ui.vbox({
        class = "music-popover",
        spacing = 4,
        width = 268,
        children = {
          ui.hbox({
            class = "music-head",
            spacing = 8,
            children = {
              ui.image({
                class = "music-cover",
                bind = "music.cover",
                fit = "scale-down",
                width = 80,
                height = 60,
                can_shrink = false,
              }),
              ui.vbox({
                class = "music-meta",
                spacing = 1,
                hexpand = true,
                children = {
                  ui.label({
                    class = "music-title",
                    bind = "music.title",
                    max_chars = 26,
                    ellipsize = "end",
                    format = "{value}",
                  }),
                  ui.label({
                    class = "music-artist",
                    bind = "music.artist",
                    max_chars = 30,
                    ellipsize = "end",
                    format = "{value}",
                  }),
                },
              }),
            },
          }),
          ui.slider({
            class = "music-seek",
            bind = "music.seek_percent",
            min = 0,
            max = 100,
            step = 0.2,
            scroll_step = 2,
            draw_value = false,
          }),
          ui.hbox({
            class = "music-time-row",
            spacing = 4,
            halign = "center",
            children = {
              ui.label({
                class = "music-time",
                bind = "music.position",
                format = "{value}",
                halign = "start",
              }),
              ui.label({
                class = "music-time-sep",
                text = "•",
              }),
              ui.label({
                class = "music-time",
                bind = "music.duration",
                format = "{value}",
                halign = "end",
              }),
            },
          }),
          ui.hbox({
            class = "music-controls",
            spacing = 6,
            halign = "center",
            children = {
              ui.button({
                class = "music-btn",
                text = "󰒮",
                on_click = function()
                  process.spawn("playerctl previous")
                end,
              }),
              ui.button({
                classes = { "music-btn", "play" },
                bind = "music.play_icon",
                format = "{value}",
                on_click = function()
                  process.spawn("playerctl play-pause")
                end,
              }),
              ui.button({
                class = "music-btn",
                text = "󰒭",
                on_click = function()
                  process.spawn("playerctl next")
                end,
              }),
            },
          }),
        },
      }),
    },
  })

  return {
    widget = widget,
  }
end

return M
