use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GBox, Label};

use crate::config::{LabelProps, RichFormatRule, RichTextStyle, WidgetConfig, WidgetProps};

pub fn apply_single_markup(
    label: &Label,
    cfg: &WidgetConfig,
    value: &str,
    state: &str,
    named_tokens: &HashMap<String, String>,
    last_markup: &Rc<RefCell<String>>,
) {
    let segments = render_segments(cfg, value, state, named_tokens);
    let markup = segments
        .iter()
        .map(|segment| segment.markup.as_str())
        .collect::<String>();

    let mut last = last_markup.borrow_mut();
    if *last != markup {
        label.set_markup(&markup);
        *last = markup;
    }
}

pub fn apply_segmented(
    row: &GBox,
    cfg: &WidgetConfig,
    value: &str,
    state: &str,
    named_tokens: &HashMap<String, String>,
    last_signature: &Rc<RefCell<String>>,
) {
    let segments = render_segments(cfg, value, state, named_tokens);
    let signature = segments_signature(&segments);

    let mut last = last_signature.borrow_mut();
    if *last == signature {
        return;
    }

    while let Some(child) = row.first_child() {
        row.remove(&child);
    }

    for segment in segments {
        let piece = Label::new(None);
        piece.set_use_markup(true);
        piece.set_markup(&segment.markup);
        piece.add_css_class("label-segment");
        for class_name in segment.classes {
            piece.add_css_class(&class_name);
        }
        row.append(&piece);
    }

    *last = signature;
}

#[derive(Clone, Debug)]
struct RenderSegment {
    markup: String,
    classes: Vec<String>,
}

fn segments_signature(segments: &[RenderSegment]) -> String {
    let mut out = String::new();
    for segment in segments {
        out.push_str(&segment.markup);
        out.push('\u{1f}');
        out.push_str(&segment.classes.join(" "));
        out.push('\u{1e}');
    }
    out
}

fn render_segments(
    cfg: &WidgetConfig,
    value: &str,
    state: &str,
    named_tokens: &HashMap<String, String>,
) -> Vec<RenderSegment> {
    let WidgetProps::Label(LabelProps {
        text,
        bind,
        format,
        format_states,
        rules,
        ..
    }) = &cfg.props
    else {
        return Vec::new();
    };

    let text_value = text.as_deref().unwrap_or("");
    let default_format = if bind.is_some() { "{value}" } else { "{text}" };
    let template = format_states
        .get(state)
        .or_else(|| format_states.get("default"))
        .map(String::as_str)
        .or(format.as_deref())
        .unwrap_or(default_format);

    let mut out = Vec::new();
    let mut rest = template;

    while let Some(open_idx) = rest.find('{') {
        let (before, after_open) = rest.split_at(open_idx);
        render_literal_segments(&mut out, before, rules);

        let after_open = &after_open[1..];
        let Some(close_idx) = after_open.find('}') else {
            render_literal_segments(&mut out, "{", rules);
            render_literal_segments(&mut out, after_open, rules);
            return out;
        };

        let (key, after_close) = after_open.split_at(close_idx);
        if let Some(token_value) = lookup_token(key, value, text_value, state, named_tokens)
            && let Some(segment) = render_piece(token_value, PieceTarget::Value, Some(key), rules)
        {
            out.push(segment);
        }

        rest = &after_close[1..];
    }

    render_literal_segments(&mut out, rest, rules);
    out
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PieceTarget {
    Value,
    Literal,
}

fn render_literal_segments(out: &mut Vec<RenderSegment>, literal: &str, rules: &[RichFormatRule]) {
    for chunk in split_literal_chunks(literal) {
        if let Some(segment) = render_piece(chunk, PieceTarget::Literal, None, rules) {
            out.push(segment);
        }
    }
}

fn split_literal_chunks(literal: &str) -> Vec<&str> {
    if literal.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;
    let mut iter = literal.char_indices();
    let (_, first_char) = iter.next().unwrap_or((0, '\0'));
    let mut last_is_word = is_word_char(first_char);

    for (idx, ch) in iter {
        let is_word = is_word_char(ch);
        if is_word != last_is_word {
            chunks.push(&literal[start..idx]);
            start = idx;
            last_is_word = is_word;
        }
    }

    chunks.push(&literal[start..]);
    chunks
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '-'
}

fn render_piece(
    raw_value: &str,
    target: PieceTarget,
    token_key: Option<&str>,
    rules: &[RichFormatRule],
) -> Option<RenderSegment> {
    if raw_value.is_empty() {
        return None;
    }

    let style = resolve_piece_style(rules, raw_value, target, token_key);
    let formatted = render_rule_format(style.as_ref().and_then(|s| s.format.as_deref()), raw_value);
    let escaped = glib::markup_escape_text(&formatted);

    let markup = if let Some(style) = style.as_ref() {
        let attrs = span_attrs(style);
        if attrs.is_empty() {
            escaped.to_string()
        } else {
            format!("<span {}>{}</span>", attrs, escaped)
        }
    } else {
        escaped.to_string()
    };

    let mut classes = style
        .as_ref()
        .and_then(|s| s.class_name.as_deref())
        .map(|v| {
            v.split_whitespace()
                .filter(|c| !c.is_empty())
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    classes.sort();
    classes.dedup();

    Some(RenderSegment { markup, classes })
}

fn render_rule_format(format: Option<&str>, value: &str) -> String {
    let template = format.unwrap_or("{value}");
    template.replace("{value}", value)
}

fn resolve_piece_style(
    rules: &[RichFormatRule],
    value: &str,
    target: PieceTarget,
    token_key: Option<&str>,
) -> Option<RichTextStyle> {
    let mut merged = RichTextStyle::default();
    let mut matched = false;

    for rule in rules {
        if !matches_rule(rule, value, target, token_key) {
            continue;
        }
        matched = true;
        merge_style(&mut merged, &rule.style);
    }

    if matched { Some(merged) } else { None }
}

fn merge_style(dst: &mut RichTextStyle, src: &RichTextStyle) {
    if src.format.is_some() {
        dst.format = src.format.clone();
    }
    if src.class_name.is_some() {
        dst.class_name = src.class_name.clone();
    }
    if src.color.is_some() {
        dst.color = src.color.clone();
    }
    if src.background.is_some() {
        dst.background = src.background.clone();
    }
    if src.weight.is_some() {
        dst.weight = src.weight.clone();
    }
    if src.style.is_some() {
        dst.style = src.style.clone();
    }
    if src.underline.is_some() {
        dst.underline = src.underline.clone();
    }
    if src.font.is_some() {
        dst.font = src.font.clone();
    }
    if src.size.is_some() {
        dst.size = src.size.clone();
    }
    if src.rise.is_some() {
        dst.rise = src.rise.clone();
    }
    if src.alpha.is_some() {
        dst.alpha = src.alpha.clone();
    }
    if src.strikethrough.is_some() {
        dst.strikethrough = src.strikethrough;
    }
}

fn matches_rule(
    rule: &RichFormatRule,
    value: &str,
    target: PieceTarget,
    token_key: Option<&str>,
) -> bool {
    if !match_target(rule.target.as_deref(), target) {
        return false;
    }

    if let Some(token_pattern) = rule.token.as_deref() {
        let Some(key) = token_key else {
            return false;
        };
        if !wildcard_match(token_pattern, key) {
            return false;
        }
    }

    if let Some(pattern) = rule.r#match.as_deref()
        && !wildcard_match(pattern, value)
    {
        return false;
    }

    if rule.min.is_some() || rule.max.is_some() {
        let Ok(n) = value.trim().parse::<f64>() else {
            return false;
        };
        if !matches_range(n, rule.min, rule.max) {
            return false;
        }
    }

    true
}

fn match_target(rule_target: Option<&str>, target: PieceTarget) -> bool {
    match rule_target.unwrap_or("any") {
        "any" => true,
        "value" => target == PieceTarget::Value,
        "literal" => target == PieceTarget::Literal,
        _ => false,
    }
}

fn lookup_token<'a>(
    key: &str,
    value: &'a str,
    text: &'a str,
    state: &'a str,
    named_tokens: &'a HashMap<String, String>,
) -> Option<&'a str> {
    match key {
        "value" => Some(value),
        "text" => Some(text),
        "state" => Some(state),
        _ => named_tokens.get(key).map(String::as_str),
    }
}

fn matches_range(value: f64, min: Option<f64>, max: Option<f64>) -> bool {
    let min_ok = min.map(|m| value >= m).unwrap_or(true);
    let max_ok = max.map(|m| value <= m).unwrap_or(true);
    min_ok && max_ok
}

fn wildcard_match(pattern: &str, input: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = input.chars().collect();
    let mut pi = 0usize;
    let mut si = 0usize;
    let mut star_pi: Option<usize> = None;
    let mut star_si = 0usize;

    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star_pi = Some(pi);
            pi += 1;
            star_si = si;
        } else if let Some(saved_pi) = star_pi {
            pi = saved_pi + 1;
            star_si += 1;
            si = star_si;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }

    pi == p.len()
}

fn span_attrs(style: &RichTextStyle) -> String {
    let mut attrs = Vec::new();
    if let Some(v) = &style.color {
        attrs.push(format!("foreground=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.background {
        attrs.push(format!("background=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.weight {
        attrs.push(format!("weight=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.style {
        attrs.push(format!("style=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.underline {
        attrs.push(format!("underline=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.font {
        attrs.push(format!("font_desc=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.size {
        attrs.push(format!("size=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.rise {
        attrs.push(format!("rise=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = &style.alpha {
        attrs.push(format!("alpha=\"{}\"", escape_attr(v)));
    }
    if let Some(v) = style.strikethrough {
        attrs.push(format!(
            "strikethrough=\"{}\"",
            if v { "true" } else { "false" }
        ));
    }
    attrs.join(" ")
}

fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
