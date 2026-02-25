use std::collections::HashMap;

pub fn render(
    format: Option<&str>,
    format_states: &HashMap<String, String>,
    state: &str,
    tokens: &[(&str, String)],
) -> String {
    let template = format_states
        .get(state)
        .or_else(|| format_states.get("default"))
        .map(String::as_str)
        .or(format)
        .unwrap_or("{label}");

    render_template(template, tokens)
}

fn render_template(template: &str, tokens: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(open_idx) = rest.find('{') {
        let (before, after_open) = rest.split_at(open_idx);
        out.push_str(before);

        let after_open = &after_open[1..];
        let Some(close_idx) = after_open.find('}') else {
            out.push('{');
            out.push_str(after_open);
            return out;
        };

        let (key, after_close) = after_open.split_at(close_idx);
        if let Some((_, value)) = tokens.iter().find(|(token_key, _)| *token_key == key) {
            out.push_str(value);
        }

        rest = &after_close[1..];
    }

    out.push_str(rest);
    out
}
