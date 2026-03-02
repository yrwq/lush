use super::types::{WidgetBase, WidgetProps};

impl WidgetBase {
    pub fn present_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.output.is_some() {
            fields.push("output");
        }
        if self.visible.is_some() {
            fields.push("visible");
        }
        if self.visible_bind.is_some() {
            fields.push("visible_bind");
        }
        if self.width.is_some() {
            fields.push("width");
        }
        if self.height.is_some() {
            fields.push("height");
        }
        if self.hexpand.is_some() {
            fields.push("hexpand");
        }
        if self.vexpand.is_some() {
            fields.push("vexpand");
        }
        if self.halign.is_some() {
            fields.push("halign");
        }
        if self.valign.is_some() {
            fields.push("valign");
        }
        if self.class_bind.is_some() {
            fields.push("class_bind");
        }
        if !self.classes.is_empty() {
            fields.push("classes");
        }
        if !self.children.is_empty() {
            fields.push("children");
        }
        fields
    }
}

impl WidgetProps {
    pub fn present_fields(&self) -> Vec<&'static str> {
        let mut fields = Vec::new();

        match self {
            Self::HBox(props) | Self::VBox(props) | Self::CenterBox(props) => {
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
            }
            Self::Revealer(props) => {
                if props.reveal.is_some() {
                    fields.push("reveal");
                }
                if props.reveal_bind.is_some() {
                    fields.push("reveal_bind");
                }
                if props.transition.is_some() {
                    fields.push("transition");
                }
                if props.duration.is_some() {
                    fields.push("duration");
                }
            }
            Self::Scroll(props) => {
                if props.h_policy.is_some() {
                    fields.push("h_policy");
                }
                if props.v_policy.is_some() {
                    fields.push("v_policy");
                }
                if props.overlay_scrolling.is_some() {
                    fields.push("overlay_scrolling");
                }
                if props.kinetic_scrolling.is_some() {
                    fields.push("kinetic_scrolling");
                }
                if props.propagate_natural_width.is_some() {
                    fields.push("propagate_natural_width");
                }
                if props.propagate_natural_height.is_some() {
                    fields.push("propagate_natural_height");
                }
                if props.min_content_width.is_some() {
                    fields.push("min_content_width");
                }
                if props.min_content_height.is_some() {
                    fields.push("min_content_height");
                }
            }
            Self::Label(props) => {
                if props.text.is_some() {
                    fields.push("text");
                }
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if !props.binds.is_empty() {
                    fields.push("binds");
                }
                if props.on_click.has_any() {
                    fields.push("on_click");
                }
                if props.format.is_some() {
                    fields.push("format");
                }
                if !props.format_states.is_empty() {
                    fields.push("format_states");
                }
                if !props.rules.is_empty() {
                    fields.push("rules");
                }
                if props.max_chars.is_some() {
                    fields.push("max_chars");
                }
                if props.ellipsize.is_some() {
                    fields.push("ellipsize");
                }
            }
            Self::Button(props) => {
                if props.text.is_some() {
                    fields.push("text");
                }
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.on_click.has_any() {
                    fields.push("on_click");
                }
                if props.format.is_some() {
                    fields.push("format");
                }
                if !props.format_states.is_empty() {
                    fields.push("format_states");
                }
                if props.angle.is_some() {
                    fields.push("angle");
                }
            }
            Self::Clock(props) => {
                if props.format.is_some() {
                    fields.push("format");
                }
                if props.display_format.is_some() {
                    fields.push("display_format");
                }
                if !props.format_states.is_empty() {
                    fields.push("format_states");
                }
                if props.interval.is_some() {
                    fields.push("interval");
                }
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
                if props.angle.is_some() {
                    fields.push("angle");
                }
            }
            Self::Entry(props) => {
                if props.text.is_some() {
                    fields.push("text");
                }
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.input_bind.is_some() {
                    fields.push("input_bind");
                }
                if props.activate_bind.is_some() {
                    fields.push("activate_bind");
                }
                if props.placeholder.is_some() {
                    fields.push("placeholder");
                }
                if props.max_chars.is_some() {
                    fields.push("max_chars");
                }
                if props.autofocus.is_some() {
                    fields.push("autofocus");
                }
            }
            Self::Workspaces(props) => {
                if props.count.is_some() {
                    fields.push("count");
                }
                if props.active_only.is_some() {
                    fields.push("active_only");
                }
                if props.all_outputs.is_some() {
                    fields.push("all_outputs");
                }
                if props.output.is_some() {
                    fields.push("output");
                }
                if props.orientation.is_some() {
                    fields.push("orientation");
                }
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
                if props.format.is_some() {
                    fields.push("format");
                }
                if !props.format_states.is_empty() {
                    fields.push("format_states");
                }
                if !props.labels.is_empty() {
                    fields.push("labels");
                }
                if !props.state_labels.is_empty() {
                    fields.push("state_labels");
                }
                if !props.format_icons.is_empty() {
                    fields.push("format_icons");
                }
                if props.show_clients.is_some() {
                    fields.push("show_clients");
                }
                if props.clients_max_items.is_some() {
                    fields.push("clients_max_items");
                }
                if props.clients_icon_size.is_some() {
                    fields.push("clients_icon_size");
                }
                if !props.clients_rules.is_empty() {
                    fields.push("clients_rules");
                }
                if props.clients_use_glyphs.is_some() {
                    fields.push("clients_use_glyphs");
                }
                if props.clients_glyph_fallback.is_some() {
                    fields.push("clients_glyph_fallback");
                }
                if props.clients_spacing.is_some() {
                    fields.push("clients_spacing");
                }
                if props.angle.is_some() {
                    fields.push("angle");
                }
            }
            Self::Dock(props) => {
                if props.orientation.is_some() {
                    fields.push("orientation");
                }
                if props.output.is_some() {
                    fields.push("output");
                }
                if props.all_outputs.is_some() {
                    fields.push("all_outputs");
                }
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
                if props.max_items.is_some() {
                    fields.push("max_items");
                }
                if props.format.is_some() {
                    fields.push("format");
                }
                if !props.format_states.is_empty() {
                    fields.push("format_states");
                }
                if !props.image_map.is_empty() {
                    fields.push("image_map");
                }
                if props.icon_size.is_some() {
                    fields.push("icon_size");
                }
                if props.on_click.has_any() {
                    fields.push("on_click");
                }
                if props.angle.is_some() {
                    fields.push("angle");
                }
            }
            Self::Tray(props) => {
                if props.orientation.is_some() {
                    fields.push("orientation");
                }
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
                if props.icon_size.is_some() {
                    fields.push("icon_size");
                }
                if props.max_items.is_some() {
                    fields.push("max_items");
                }
                if props.show_passive.is_some() {
                    fields.push("show_passive");
                }
                if props.hide_when_empty.is_some() {
                    fields.push("hide_when_empty");
                }
            }
            Self::Image(props) => {
                if props.path.is_some() {
                    fields.push("path");
                }
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.fit.is_some() {
                    fields.push("fit");
                }
                if props.can_shrink.is_some() {
                    fields.push("can_shrink");
                }
                if props.on_click.has_any() {
                    fields.push("on_click");
                }
            }
            Self::Progress(props) => {
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.value.is_some() {
                    fields.push("value");
                }
                if props.min.is_some() {
                    fields.push("min");
                }
                if props.max.is_some() {
                    fields.push("max");
                }
                if props.inverted.is_some() {
                    fields.push("inverted");
                }
            }
            Self::Slider(props) => {
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.value.is_some() {
                    fields.push("value");
                }
                if props.min.is_some() {
                    fields.push("min");
                }
                if props.max.is_some() {
                    fields.push("max");
                }
                if props.step.is_some() {
                    fields.push("step");
                }
                if props.scroll_step.is_some() {
                    fields.push("scroll_step");
                }
                if props.orientation.is_some() {
                    fields.push("orientation");
                }
                if props.inverted.is_some() {
                    fields.push("inverted");
                }
                if props.draw_value.is_some() {
                    fields.push("draw_value");
                }
                if props.digits.is_some() {
                    fields.push("digits");
                }
            }
            Self::List(props) => {
                if props.bind.is_some() {
                    fields.push("bind");
                }
                if props.count.is_some() {
                    fields.push("count");
                }
                if props.orientation.is_some() {
                    fields.push("orientation");
                }
                if props.spacing.is_some() {
                    fields.push("spacing");
                }
            }
            Self::Popover(props) => {
                if props.position.is_some() {
                    fields.push("position");
                }
                if props.autohide.is_some() {
                    fields.push("autohide");
                }
                if props.has_arrow.is_some() {
                    fields.push("has_arrow");
                }
            }
            Self::Overlay(_) => {}
        }

        fields
    }
}
