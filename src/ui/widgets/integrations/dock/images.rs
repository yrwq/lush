use std::collections::HashMap;
use std::path::Path;

use crate::runtime::compositor::ToplevelEntry;

pub(super) enum ImageSource {
    File(String),
    IconName(String),
}

pub(super) fn resolve_image_source(
    entry: &ToplevelEntry,
    image_map: &HashMap<String, String>,
) -> Option<ImageSource> {
    let app_id = entry.app_id.trim();
    let title = entry.title.trim();

    for (pattern, value) in image_map {
        if wildcard_match(pattern, app_id) || wildcard_match(pattern, title) {
            return parse_image_source(value);
        }
    }

    for candidate in app_id_candidates(app_id) {
        if let Some(src) = parse_image_source(candidate) {
            return Some(src);
        }
    }

    None
}

fn parse_image_source(value: &str) -> Option<ImageSource> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed.contains('/') || Path::new(trimmed).is_absolute() {
        return Some(ImageSource::File(trimmed.to_string()));
    }

    Some(ImageSource::IconName(trimmed.to_string()))
}

fn app_id_candidates(app_id: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let trimmed = app_id.trim();
    if trimmed.is_empty() {
        return out;
    }

    out.push(trimmed);

    if let Some(last) = trimmed.rsplit('.').next()
        && !last.is_empty()
        && last != trimmed
    {
        out.push(last);
    }

    out
}

fn wildcard_match(pattern: &str, input: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let p = pattern.as_bytes();
    let s = input.as_bytes();

    let (mut pi, mut si) = (0_usize, 0_usize);
    let (mut star, mut match_i) = (None::<usize>, 0_usize);

    while si < s.len() {
        if pi < p.len() && (p[pi] == s[si] || p[pi] == b'?') {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            match_i = si;
        } else if let Some(star_idx) = star {
            pi = star_idx + 1;
            match_i += 1;
            si = match_i;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}
