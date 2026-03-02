local M = {}

function M.create(opts)
  opts = opts or {}
  local lush = opts.lush or require("lush")
  local ui = opts.ui or lush.ui
  local scheduler = lush.scheduler

  local name = opts.name or "control-center"
  local width = opts.width or 320
  local margin_top = opts.margin_top or 10
  local margin_right = opts.margin_right or 8

  local volume_apply_timer = nil

  local function cancel_timer(timer_id)
    if timer_id ~= nil then
      scheduler.cancel(timer_id)
    end
    return nil
  end

  lush.state.watch("control.audio.volume.input", function(value)
    local sink_name = tostring(lush.state.get("data.audio.sink", "") or "")
    if sink_name == "" then
      return
    end
    local n = tonumber(value)
    if n == nil then
      return
    end
    local clamped = math.max(0, math.min(150, math.floor(n + 0.5)))
    volume_apply_timer = cancel_timer(volume_apply_timer)
    volume_apply_timer = scheduler.after(0.08, function()
      lush.audio.set_volume(clamped)
      volume_apply_timer = nil
    end)
  end, { immediate = false })

  local window = ui.window({
    name = name,
    visible = false,
    layer = "overlay",
    exclusive = false,
    anchor = { "top", "right" },
    margin_top = margin_top,
    margin_right = margin_right,
    width = width,
    root = ui.vbox({
      class = "control-center-panel",
      spacing = 12,
      width = width,
      children = {
        ui.label({
          class = "control-center-title",
          text = "Control Center",
          halign = "start",
        }),
        ui.vbox({
          class = "control-card",
          spacing = 8,
          children = {
            ui.hbox({
              class = "control-volume-row",
              spacing = 8,
              children = {
                ui.button({
                  class = "control-mute-button",
                  bind = "data.audio.muted",
                  format = "{value}",
                  format_states = {
                    ["1"] = "󰖁",
                    ["0"] = "󰕾",
                    default = "󰕾",
                  },
                  class_bind = "data.audio.muted",
                  on_click = function()
                    lush.audio.toggle_mute()
                  end,
                }),
                ui.slider({
                  class = "control-slider",
                  bind = "data.audio.volume",
                  input_bind = "control.audio.volume.input",
                  min = 0,
                  max = 150,
                  step = 5,
                  scroll_step = 5,
                  draw_value = false,
                  hexpand = true,
                }),
                ui.label({
                  class = "control-value",
                  binds = {
                    ["vol"] = "data.audio.volume",
                  },
                  format = "{vol}%",
                  halign = "end",
                }),
              },
            }),
          },
        }),
      },
    }),
  })

  local toggle_button = ui.button({
    class = "control-center-toggle",
    text = "󰕮",
    on_click = function()
      lush.windows.toggle(name)
    end,
  })

  return {
    window = window,
    toggle_button = toggle_button,
    name = name,
  }
end

return M
