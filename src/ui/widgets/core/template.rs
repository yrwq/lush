use crate::config::{
    ButtonProps, ClickAction, ClickBindings, ImageProps, LabelProps, ProgressProps, RevealerProps,
    WidgetConfig, WidgetProps,
};

pub fn expand_widget_template(cfg: &mut WidgetConfig, base: &str, index: usize) {
    let item_ref = format!("{}{}", base, index);
    let item_index = index.to_string();

    cfg.base.output = replace_opt(cfg.base.output.take(), base, &item_ref, &item_index);
    cfg.base.visible_bind = replace_opt(cfg.base.visible_bind.take(), base, &item_ref, &item_index);
    cfg.base.halign = replace_opt(cfg.base.halign.take(), base, &item_ref, &item_index);
    cfg.base.valign = replace_opt(cfg.base.valign.take(), base, &item_ref, &item_index);
    cfg.base.class_bind = replace_opt(cfg.base.class_bind.take(), base, &item_ref, &item_index);
    cfg.base.classes = cfg
        .base
        .classes
        .iter()
        .map(|v| replace_tokens(v, base, &item_ref, &item_index))
        .collect();

    match &mut cfg.props {
        WidgetProps::HBox(_)
        | WidgetProps::VBox(_)
        | WidgetProps::CenterBox(_)
        | WidgetProps::Scroll(_)
        | WidgetProps::Overlay(_) => {}
        WidgetProps::Revealer(props) => {
            expand_revealer_props(props, base, &item_ref, &item_index);
        }
        WidgetProps::Label(props) => expand_label_props(props, base, &item_ref, &item_index),
        WidgetProps::Button(props) => expand_button_props(props, base, &item_ref, &item_index),
        WidgetProps::Clock(props) => {
            props.format = replace_opt(props.format.take(), base, &item_ref, &item_index);
            props.display_format =
                replace_opt(props.display_format.take(), base, &item_ref, &item_index);
            props.bind = replace_opt(props.bind.take(), base, &item_ref, &item_index);
        }
        WidgetProps::Workspaces(props) => {
            props.output = replace_opt(props.output.take(), base, &item_ref, &item_index);
            props.orientation = replace_opt(props.orientation.take(), base, &item_ref, &item_index);
            props.labels = props
                .labels
                .iter()
                .map(|v| replace_tokens(v, base, &item_ref, &item_index))
                .collect();
            props.format = replace_opt(props.format.take(), base, &item_ref, &item_index);
        }
        WidgetProps::Dock(props) => {
            props.orientation = replace_opt(props.orientation.take(), base, &item_ref, &item_index);
            props.output = replace_opt(props.output.take(), base, &item_ref, &item_index);
            props.format = replace_opt(props.format.take(), base, &item_ref, &item_index);
            props.image_map = props
                .image_map
                .iter()
                .map(|(k, v)| (k.clone(), replace_tokens(v, base, &item_ref, &item_index)))
                .collect();
        }
        WidgetProps::Tray(_) => {}
        WidgetProps::Image(props) => expand_image_props(props, base, &item_ref, &item_index),
        WidgetProps::Progress(props) => expand_progress_props(props, base, &item_ref, &item_index),
        WidgetProps::Slider(props) => {
            props.bind = replace_opt(props.bind.take(), base, &item_ref, &item_index);
            props.orientation = replace_opt(props.orientation.take(), base, &item_ref, &item_index);
        }
        WidgetProps::Popover(props) => {
            props.position = replace_opt(props.position.take(), base, &item_ref, &item_index);
        }
        WidgetProps::List(props) => {
            props.bind = replace_opt(props.bind.take(), base, &item_ref, &item_index);
            props.orientation = replace_opt(props.orientation.take(), base, &item_ref, &item_index);
        }
    }

    for child in &mut cfg.base.children {
        expand_widget_template(child, base, index);
    }
}

fn expand_label_props(props: &mut LabelProps, base: &str, item_ref: &str, item_index: &str) {
    props.text = replace_opt(props.text.take(), base, item_ref, item_index);
    props.bind = replace_opt(props.bind.take(), base, item_ref, item_index);
    props.binds = props
        .binds
        .iter()
        .map(|(k, v)| (k.clone(), replace_tokens(v, base, item_ref, item_index)))
        .collect();
    for rule in &mut props.rules {
        rule.target = replace_opt(rule.target.take(), base, item_ref, item_index);
        rule.token = replace_opt(rule.token.take(), base, item_ref, item_index);
        rule.r#match = replace_opt(rule.r#match.take(), base, item_ref, item_index);
        rule.style.format = replace_opt(rule.style.format.take(), base, item_ref, item_index);
        rule.style.class_name =
            replace_opt(rule.style.class_name.take(), base, item_ref, item_index);
        rule.style.color = replace_opt(rule.style.color.take(), base, item_ref, item_index);
        rule.style.background =
            replace_opt(rule.style.background.take(), base, item_ref, item_index);
        rule.style.weight = replace_opt(rule.style.weight.take(), base, item_ref, item_index);
        rule.style.style = replace_opt(rule.style.style.take(), base, item_ref, item_index);
        rule.style.underline = replace_opt(rule.style.underline.take(), base, item_ref, item_index);
        rule.style.font = replace_opt(rule.style.font.take(), base, item_ref, item_index);
        rule.style.size = replace_opt(rule.style.size.take(), base, item_ref, item_index);
        rule.style.rise = replace_opt(rule.style.rise.take(), base, item_ref, item_index);
        rule.style.alpha = replace_opt(rule.style.alpha.take(), base, item_ref, item_index);
    }
    props.format = replace_opt(props.format.take(), base, item_ref, item_index);
    props.ellipsize = replace_opt(props.ellipsize.take(), base, item_ref, item_index);
    replace_click_shell_actions(&mut props.on_click, base, item_ref, item_index);
}

fn expand_button_props(props: &mut ButtonProps, base: &str, item_ref: &str, item_index: &str) {
    props.text = replace_opt(props.text.take(), base, item_ref, item_index);
    props.bind = replace_opt(props.bind.take(), base, item_ref, item_index);
    props.format = replace_opt(props.format.take(), base, item_ref, item_index);
    replace_click_shell_actions(&mut props.on_click, base, item_ref, item_index);
}

fn expand_image_props(props: &mut ImageProps, base: &str, item_ref: &str, item_index: &str) {
    props.path = replace_opt(props.path.take(), base, item_ref, item_index);
    props.bind = replace_opt(props.bind.take(), base, item_ref, item_index);
    props.fit = replace_opt(props.fit.take(), base, item_ref, item_index);
    replace_click_shell_actions(&mut props.on_click, base, item_ref, item_index);
}

fn expand_revealer_props(props: &mut RevealerProps, base: &str, item_ref: &str, item_index: &str) {
    props.reveal_bind = replace_opt(props.reveal_bind.take(), base, item_ref, item_index);
    props.transition = replace_opt(props.transition.take(), base, item_ref, item_index);
}

fn expand_progress_props(props: &mut ProgressProps, base: &str, item_ref: &str, item_index: &str) {
    props.bind = replace_opt(props.bind.take(), base, item_ref, item_index);
}

fn replace_opt(
    value: Option<String>,
    base: &str,
    item_ref: &str,
    item_index: &str,
) -> Option<String> {
    value.map(|v| replace_tokens(&v, base, item_ref, item_index))
}

pub fn replace_tokens(value: &str, base: &str, item_ref: &str, item_index: &str) -> String {
    value
        .replace("{item}", item_ref)
        .replace("{slot}", item_ref)
        .replace("{base}", base)
        .replace("{index}", item_index)
}

fn replace_click_shell_actions(
    bindings: &mut ClickBindings<ClickAction>,
    base: &str,
    item_ref: &str,
    item_index: &str,
) {
    replace_shell_in_item(&mut bindings.left, base, item_ref, item_index);
    replace_shell_in_item(&mut bindings.middle, base, item_ref, item_index);
    replace_shell_in_item(&mut bindings.right, base, item_ref, item_index);
    replace_shell_in_item(&mut bindings.wheel_up, base, item_ref, item_index);
    replace_shell_in_item(&mut bindings.wheel_down, base, item_ref, item_index);
}

fn replace_shell_in_item(
    action: &mut Option<ClickAction>,
    base: &str,
    item_ref: &str,
    item_index: &str,
) {
    if let Some(ClickAction::Shell(cmd)) = action.as_mut() {
        *cmd = replace_tokens(cmd, base, item_ref, item_index);
    }
}
