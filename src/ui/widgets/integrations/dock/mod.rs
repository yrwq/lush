mod actions;
mod images;
mod matching;

use std::cell::RefCell;
use std::rc::Rc;

use glib::MainContext;
use gtk4::prelude::*;
use gtk4::{
    Box as GBox, EventControllerScroll, EventControllerScrollFlags, GestureClick, Image, Label,
    Orientation, Widget,
};

use crate::config::{ClickBindings, DockClickAction, WidgetConfig, WidgetProps};
use crate::runtime::compositor;
use crate::runtime::compositor::ToplevelEntry;
use crate::ui::widgets::core::build_ctx::WidgetBuildCtx;
use crate::ui::widgets::core::common::{apply_clickable_cursor, finalize_widget, format_text};
use crate::ui::widgets::core::format::render as render_format;

#[derive(Default)]
struct DockState {
    entries: RefCell<Vec<ToplevelEntry>>,
    focused_title: RefCell<String>,
    focused_app_id: RefCell<String>,
    item_boxes: RefCell<Vec<GBox>>,
}

#[derive(Clone)]
struct DockRenderConfig {
    format: Option<String>,
    format_states: std::collections::HashMap<String, String>,
    image_map: std::collections::HashMap<String, String>,
    click_bindings: ClickBindings<DockClickAction>,
    angle: Option<f64>,
    max_items: usize,
    icon_size: i32,
}

pub fn build(cfg: &WidgetConfig, ctx: &WidgetBuildCtx<'_>) -> Widget {
    let WidgetProps::Dock(props) = &cfg.props else {
        unreachable!("dock builder received non-dock props");
    };

    let output_selector = props
        .output
        .as_deref()
        .or(ctx.default_output)
        .map(str::to_string);
    let selector_for_backend = if props.all_outputs.unwrap_or(false) {
        None
    } else {
        output_selector.as_deref().or(Some("focused"))
    };

    let Some(state_rx) = compositor::subscribe_state(selector_for_backend) else {
        log::debug!("dock: compositor toplevel backend unavailable; widget disabled");
        let placeholder: Widget = GBox::new(Orientation::Horizontal, 0).upcast();
        placeholder.set_visible(false);
        return placeholder;
    };

    let orientation = match props.orientation.as_deref() {
        Some("vertical") => Orientation::Vertical,
        _ => Orientation::Horizontal,
    };
    let container = GBox::new(orientation, props.spacing.unwrap_or(6));
    container.add_css_class("dock-container");

    let render_cfg = Rc::new(DockRenderConfig {
        format: props.format.clone(),
        format_states: props.format_states.clone(),
        image_map: props.image_map.clone(),
        click_bindings: props.on_click.clone(),
        angle: props.angle,
        max_items: props.max_items.unwrap_or(12).clamp(1, 128) as usize,
        icon_size: props.icon_size.unwrap_or(16).clamp(8, 128),
    });
    let state = Rc::new(DockState::default());

    let render_all = {
        let container = container.clone();
        let state = state.clone();
        let render_cfg = render_cfg.clone();

        move || {
            while let Some(child) = container.first_child() {
                container.remove(&child);
            }
            state.item_boxes.borrow_mut().clear();

            let windows = state.entries.borrow();
            let focused_idx = matching::focused_index(
                &windows,
                state.focused_title.borrow().as_str(),
                state.focused_app_id.borrow().as_str(),
            );

            for (idx, entry) in windows.iter().take(render_cfg.max_items).enumerate() {
                let item = GBox::new(Orientation::Horizontal, 6);
                item.add_css_class("dock-item");
                apply_clickable_cursor(&item.clone().upcast(), true);

                if focused_idx == Some(idx) {
                    item.add_css_class("focused");
                }

                if let Some(image_source) =
                    images::resolve_image_source(entry, &render_cfg.image_map)
                {
                    let image = Image::new();
                    image.add_css_class("dock-image");
                    image.set_pixel_size(render_cfg.icon_size);
                    match image_source {
                        images::ImageSource::File(path) => image.set_from_file(Some(path)),
                        images::ImageSource::IconName(name) => image.set_icon_name(Some(&name)),
                    }
                    item.append(&image);
                }

                let title = matching::display_title(entry);
                let class = entry.app_id.trim().to_string();
                let index = (idx + 1).to_string();
                let item_state = if focused_idx == Some(idx) {
                    "focused"
                } else {
                    "default"
                };
                let rendered = render_format(
                    render_cfg.format.as_deref().or(Some("")),
                    &render_cfg.format_states,
                    item_state,
                    &[
                        ("label", title.clone()),
                        ("title", title),
                        ("class", class.clone()),
                        ("app_id", class),
                        ("identifier", entry.identifier.clone()),
                        ("index", index),
                        ("state", item_state.to_string()),
                    ],
                );

                if !rendered.trim().is_empty() {
                    let text = Label::new(None);
                    text.add_css_class("dock-text");
                    text.set_text(&format_text(&rendered, render_cfg.angle));
                    item.append(&text);
                }

                let title_for_click = entry.title.clone();
                let app_id_for_click = entry.app_id.clone();
                let duplicate_rank = matching::duplicate_rank_in_prefix(&windows, idx, entry);
                let click_bindings_for_item = render_cfg.click_bindings.clone();
                let click = GestureClick::new();
                click.set_button(0);
                click.connect_pressed(move |gesture, _, _, _| {
                    let button = gesture.current_button();
                    let result = match button {
                        1 => actions::run(
                            click_bindings_for_item
                                .left
                                .as_ref()
                                .unwrap_or(&DockClickAction::Activate),
                            &app_id_for_click,
                            &title_for_click,
                            duplicate_rank,
                        ),
                        2 => actions::run(
                            click_bindings_for_item
                                .middle
                                .as_ref()
                                .unwrap_or(&DockClickAction::None),
                            &app_id_for_click,
                            &title_for_click,
                            duplicate_rank,
                        ),
                        3 => actions::run(
                            click_bindings_for_item
                                .right
                                .as_ref()
                                .unwrap_or(&DockClickAction::None),
                            &app_id_for_click,
                            &title_for_click,
                            duplicate_rank,
                        ),
                        _ => Ok(()),
                    };
                    if let Err(err) = result {
                        log::debug!("dock action failed: {}", err);
                    }
                });
                item.add_controller(click);

                let app_id_for_scroll = entry.app_id.clone();
                let title_for_scroll = entry.title.clone();
                let duplicate_rank_for_scroll = duplicate_rank;
                let click_bindings_for_scroll = render_cfg.click_bindings.clone();
                let scroll = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
                scroll.connect_scroll(move |_, _, dy| {
                    let action = if dy < 0.0 {
                        click_bindings_for_scroll.wheel_up.as_ref()
                    } else if dy > 0.0 {
                        click_bindings_for_scroll.wheel_down.as_ref()
                    } else {
                        None
                    };

                    if let Some(action) = action {
                        if let Err(err) = actions::run(
                            action,
                            &app_id_for_scroll,
                            &title_for_scroll,
                            duplicate_rank_for_scroll,
                        ) {
                            log::debug!("dock wheel action failed: {}", err);
                        }
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                });
                item.add_controller(scroll);

                container.append(&item);
                state.item_boxes.borrow_mut().push(item);
            }
        }
    };

    let state_for_updates = state.clone();
    let render_for_updates = render_all.clone();
    MainContext::default().spawn_local(async move {
        while let Ok(snapshot) = state_rx.recv().await {
            let mut needs_render = false;
            {
                let mut entries = state_for_updates.entries.borrow_mut();
                if *entries != snapshot.toplevels {
                    *entries = snapshot.toplevels.clone();
                    needs_render = true;
                }
            }
            *state_for_updates.focused_title.borrow_mut() = snapshot.workspace.focused_window.title;
            *state_for_updates.focused_app_id.borrow_mut() =
                snapshot.workspace.focused_window.app_id;

            if needs_render {
                render_for_updates();
                continue;
            }
            let focused_idx = matching::focused_index(
                &state_for_updates.entries.borrow(),
                state_for_updates.focused_title.borrow().as_str(),
                state_for_updates.focused_app_id.borrow().as_str(),
            );
            matching::apply_focus_classes(&state_for_updates.item_boxes.borrow(), focused_idx);
        }
    });

    render_all();

    let widget: Widget = container.upcast();
    finalize_widget(&widget, cfg, ctx.bus, true);
    widget
}
