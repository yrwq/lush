local M = {}

function M.create(opts)
  opts = opts or {}
  local lush = opts.lush or require("lush")
  local ui = opts.ui or lush.ui

  local names = {
    popup = opts.popup_name or "notif-popup",
    center = opts.center_name or "notif-center",
  }

  local popup_count = opts.popup_count or 3
  local history_count = opts.history_count or 24

  local function popup_row()
    return ui.hbox({
      class = "notif",
      visible = false,
      spacing = 8,
      visible_bind = "{slot}.visible",
      class_bind = "{slot}.urgency_name",
      children = {
        ui.image({
          bind = "{slot}.icon",
          visible_bind = "{slot}.icon",
          class = "notif-img",
          width = 48,
          height = 32,
          fit = "contain",
          valign = "center",
        }),
        ui.vbox({
          hexpand = true,
          halign = "start",
          spacing = 2,
          children = {
            ui.label({
              bind = "{slot}.title",
              class = "notif-title",
              halign = "start",
              max_chars = 30,
              ellipsize = "end",
            }),
            ui.label({
              bind = "{slot}.body",
              class = "notif-body",
              halign = "start",
              max_chars = 30,
              ellipsize = "end",
            }),
          },
        }),
      },
    })
  end

  local function history_row()
    return ui.hbox({
      classes = { "notif", "notif-row-history" },
      visible = false,
      visible_bind = "{slot}.visible",
      class_bind = "{slot}.urgency_name",
      spacing = 8,
      children = {
        ui.image({
          bind = "{slot}.icon",
          visible_bind = "{slot}.icon",
          class = "notif-img",
          width = 48,
          height = 32,
          fit = "contain",
          valign = "center",
        }),
        ui.vbox({
          hexpand = true,
          halign = "start",
          spacing = 2,
          children = {
            ui.label({
              bind = "{slot}.title",
              class = "notif-title",
              halign = "start",
              max_chars = 30,
              ellipsize = "end",
            }),
            ui.label({
              bind = "{slot}.body",
              class = "notif-body",
              halign = "start",
              max_chars = 30,
              ellipsize = "end",
            }),
          },
        }),
        ui.button({
          text = "",
          class = "notif-delete",
          on_click = "lush.notifications.delete:{index}",
        }),
      },
    })
  end

  local popup_window = ui.window({
    name = names.popup,
    visible = false,
    anchor = { "top", "right" },
    layer = "overlay",
    exclusive = false,
    margin_top = 10,
    margin_right = 10,
    root = ui.list({
      class = "notif-stack",
      bind = "notification.slot",
      count = popup_count,
      spacing = 8,
      children = { popup_row() },
    }),
  })

  local center_window = ui.window({
    name = names.center,
    visible = false,
    anchor = { "top", "right" },
    layer = "overlay",
    exclusive = false,
    margin_top = 10,
    margin_right = 8,
    root = ui.vbox({
      class = "notif-history-panel",
      spacing = 8,
      width = 400,
      children = {
        ui.hbox({
          class = "notif-history-header",
          spacing = 8,
          children = {
            ui.label({
              text = "Notifications",
              class = "notif-history-title",
              hexpand = true,
              halign = "start",
            }),
            ui.button({
              text = "clear all",
              class = "notif-clear-all",
              on_click = function()
                lush.notifications.clear()
              end,
            }),
          },
        }),
        ui.scroll({
          class = "notif-history-scroll",
          height = 420,
          h_policy = "never",
          v_policy = "automatic",
          overlay_scrolling = false,
          propagate_natural_width = true,
          children = {
            ui.list({
              classes = { "notif-stack", "notif-history-stack" },
              bind = "notification.history",
              count = history_count,
              spacing = 4,
              children = { history_row() },
            }),
          },
        }),
      },
    }),
  })

  lush.signal.on("notification.visible", function(value)
    lush.windows.set_visible(names.popup, value == "1")
  end)

  local bell_button = ui.button({
    text = "",
    class = opts.bell_class or "notif-bell",
    on_click = function()
      lush.windows.toggle(names.center)
    end,
  })

  return {
    names = names,
    windows = { popup_window, center_window },
    widgets = {
      bell_button = bell_button
    },
  }
end

return M
