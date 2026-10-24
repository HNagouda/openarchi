//! Reads the colours out of whatever a palette generator exports.
//!
//! The text is untrusted: a pasted SVG is scanned for its `<rect>` tags, never
//! rendered or run, and a name is kept only as a short plain string. The kind
//! of text is told from how it starts, so each format has one reader:
//!
//! ```text
//! <svg ...><rect fill="#f79f79" x="0" .../>...</svg>     swatches, left to right by x
//! <palette><color name="..." hex="f79f79" .../></palette>
//! ["f79f79", ...]  {"Tangerine Dream": "f79f79", ...}  [{"name": "...", "hex": "f79f79"}, ...]
//! https://.../f79f79-f7d08a-e3f09b-87b6a7-5b5941
//! f79f79,f7d08a,...   #f79f79, #f7d08a, ...   one per line
//! ```
//!
//! Anything else that holds "#rrggbb" codes, such as CSS variables, falls
//! back to those codes in the order they appear.

mod json;

use super::Swatch;
use crate::{Error, Result};

pub const MIN_SWATCHES: usize = 2;
pub const MAX_SWATCHES: usize = 10;
/// A palette export is a few kilobytes; anything far larger is not one.
const MAX_INPUT_BYTES: usize = 512 * 1024;
/// Names longer than this are cut, so a hostile file cannot flood the editor.
const MAX_NAME_CHARS: usize = 40;
/// Characters that split a plain list of codes.
const LIST_SEPARATORS: [char; 6] = [',', ';', ' ', '\n', '\t', '\r'];
const EXAMPLE: &str = "paste hex codes like \"f79f79, f7d08a, e3f09b\", or a palette generator's SVG, JSON, XML or link";

/// The swatches in `text`, in the palette's own order, between 2 and 10 of them.
pub fn parse(text: &str) -> Result<Vec<Swatch>> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(invalid(format!(
            "the text is {} KB, which is too large for a palette",
            text.len() / 1024
        )));
    }
    let trimmed = text.trim().trim_start_matches('\u{feff}');
    if trimmed.is_empty() {
        return Err(invalid(format!("there is nothing to read; {EXAMPLE}")));
    }

    let swatches = if trimmed.starts_with('<') {
        markup(trimmed)?
    } else if trimmed.starts_with('[') || trimmed.starts_with('{') {
        json::read(trimmed)?
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        link(trimmed)?
    } else {
        list(trimmed)?
    };

    check_count(swatches)
}

/// "f79f79", "#F79F79", "fa7" or "#fa7" -> "#F79F79" / "#FFAA77".
pub fn hex_code(token: &str) -> Option<String> {
    let digits = token.trim().trim_start_matches('#');
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match digits.len() {
        6 => Some(format!("#{}", digits.to_ascii_uppercase())),
        3 => {
            let doubled: String = digits.chars().flat_map(|c| [c, c]).collect();
            Some(format!("#{}", doubled.to_ascii_uppercase()))
        }
        _ => None,
    }
}

fn check_count(swatches: Vec<Swatch>) -> Result<Vec<Swatch>> {
    if swatches.is_empty() {
        return Err(invalid(format!("no colours were found; {EXAMPLE}")));
    }
    if swatches.len() < MIN_SWATCHES {
        return Err(invalid(format!(
            "found only 1 colour ({}); a palette needs at least {MIN_SWATCHES}",
            swatches[0].hex
        )));
    }
    if swatches.len() > MAX_SWATCHES {
        return Err(invalid(format!(
            "found {} colours; a palette holds at most {MAX_SWATCHES}, so remove some first",
            swatches.len()
        )));
    }

    Ok(swatches)
}

// * ====================================================================
// * === markup: SVG swatches and XML palettes
// * ====================================================================

fn markup(text: &str) -> Result<Vec<Swatch>> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("<svg") {
        return svg(text);
    }
    if lower.contains("<color") {
        return xml(text);
    }
    Err(invalid(
        "this looks like markup, but has neither <rect> swatches nor <color> entries".to_string(),
    ))
}

/// Every `<rect>` with a colour fill, sorted left to right, as the swatches are drawn.
fn svg(text: &str) -> Result<Vec<Swatch>> {
    let mut placed: Vec<(f64, Swatch)> = Vec::new();
    for tag in tags(text, "rect") {
        let fill = attribute(tag, "fill").or_else(|| style_fill(tag));
        let Some(hex) = fill.as_deref().and_then(hex_code) else {
            // Backgrounds with fill="none" or a gradient are not swatches.
            continue;
        };
        let x = attribute(tag, "x")
            .and_then(|value| value.trim_end_matches('%').parse::<f64>().ok())
            .unwrap_or(0.0);
        placed.push((x, swatch(hex, None)));
    }
    if placed.is_empty() {
        return Err(invalid(
            "the SVG has no <rect> swatches with a fill like \"#f79f79\"".to_string(),
        ));
    }

    // Stable, so swatches without an x keep the order they are written in.
    placed.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(placed.into_iter().map(|(_, swatch)| swatch).collect())
}

/// `<color name="..." hex="f79f79" r="247" g="159" b="121" />`, by hex or by r, g and b.
fn xml(text: &str) -> Result<Vec<Swatch>> {
    let mut swatches = Vec::new();
    for tag in tags(text, "color") {
        let from_channels = || {
            let channel = |name: &str| attribute(tag, name)?.trim().parse::<u8>().ok();
            Some(format!(
                "#{:02X}{:02X}{:02X}",
                channel("r")?,
                channel("g")?,
                channel("b")?
            ))
        };
        let Some(hex) = attribute(tag, "hex")
            .as_deref()
            .and_then(hex_code)
            .or_else(from_channels)
        else {
            return Err(invalid(format!(
                "a <color> entry has neither a hex code nor r, g and b: {}",
                excerpt(tag)
            )));
        };
        swatches.push(swatch(hex, attribute(tag, "name")));
    }

    Ok(swatches)
}

/// The insides of every `<name ...>` tag, up to its closing `>`.
fn tags<'a>(text: &'a str, name: &str) -> Vec<&'a str> {
    let opening = format!("<{name}");
    let lower = text.to_ascii_lowercase();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(offset) = lower[from..].find(&opening) {
        let start = from + offset + opening.len();
        // "<rectangle" is not "<rect".
        let is_whole_name = text[start..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace() || c == '/' || c == '>');
        let end = text[start..]
            .find('>')
            .map_or(text.len(), |close| start + close);
        if is_whole_name {
            found.push(&text[start..end]);
        }
        from = end;
    }

    found
}

/// The value of `name="..."` or `name='...'` inside one tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(offset) = lower[from..].find(name) {
        let start = from + offset;
        from = start + name.len();
        let is_word_start = start == 0 || lower[..start].ends_with(|c: char| c.is_whitespace());
        let rest = tag[from..].trim_start();
        let Some(rest) = rest.strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            continue;
        };
        if !is_word_start {
            continue;
        }
        let value = &rest[1..];
        return value.find(quote).map(|close| value[..close].to_string());
    }

    None
}

/// `style="fill:#f79f79"`, which some editors write instead of a fill attribute.
fn style_fill(tag: &str) -> Option<String> {
    let style = attribute(tag, "style")?;
    style.split(';').find_map(|declaration| {
        let (property, value) = declaration.split_once(':')?;
        (property.trim() == "fill").then(|| value.trim().to_string())
    })
}

// * ====================================================================
// * === links and plain lists
// * ====================================================================

/// The last part of the path, "f79f79-f7d08a-...", without a query or fragment.
fn link(text: &str) -> Result<Vec<Swatch>> {
    let path = text.split(['?', '#']).next().unwrap_or_default();
    let last = path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let codes: Vec<Option<String>> = last.split('-').map(hex_code).collect();
    if codes.len() < MIN_SWATCHES || codes.iter().any(Option::is_none) {
        return Err(invalid(format!(
            "the link does not end in hex codes joined by dashes, like .../f79f79-f7d08a-e3f09b; it ends in {:?}",
            excerpt(last)
        )));
    }

    Ok(codes
        .into_iter()
        .flatten()
        .map(|hex| swatch(hex, None))
        .collect())
}

/// Codes split by commas, spaces or lines, each with or without "#". When a
/// token is not a code, every "#rrggbb" in the text is taken instead, which
/// reads CSS and other code a generator copies out.
fn list(text: &str) -> Result<Vec<Swatch>> {
    let tokens: Vec<&str> = text
        .split(LIST_SEPARATORS)
        .map(|token| token.trim().trim_matches(['"', '\'']))
        .filter(|token| !token.is_empty())
        .collect();
    let codes: Vec<Option<String>> = tokens.iter().map(|token| hex_code(token)).collect();
    if codes.iter().all(Option::is_some) {
        return Ok(codes
            .into_iter()
            .flatten()
            .map(|hex| swatch(hex, None))
            .collect());
    }

    let hashed = hashed_codes(text);
    if !hashed.is_empty() {
        return Ok(hashed.into_iter().map(|hex| swatch(hex, None)).collect());
    }
    let first_bad = tokens
        .iter()
        .zip(&codes)
        .find(|(_, code)| code.is_none())
        .map(|(token, _)| *token)
        .unwrap_or_default();
    Err(invalid(format!(
        "{:?} is not a hex colour; {EXAMPLE}",
        excerpt(first_bad)
    )))
}

/// Every "#rrggbb" or "#rgb" standing on its own, in order.
fn hashed_codes(text: &str) -> Vec<String> {
    let mut codes = Vec::new();
    for (start, _) in text.match_indices('#') {
        let digits: String = text[start + 1..]
            .chars()
            .take_while(char::is_ascii_hexdigit)
            .collect();
        if let Some(hex) = hex_code(&digits) {
            codes.push(hex);
        }
    }

    codes
}

// * ====================================================================
// * === utils
// * ====================================================================

pub(super) fn swatch(hex: String, name: Option<String>) -> Swatch {
    let name = name
        .map(|name| {
            name.chars()
                .filter(|c| !c.is_control())
                .take(MAX_NAME_CHARS)
                .collect::<String>()
                .trim()
                .to_string()
        })
        .filter(|name| !name.is_empty());
    Swatch { hex, name }
}

/// The start of a bad value, short enough to quote in an error.
pub(super) fn excerpt(text: &str) -> String {
    const EXCERPT_CHARS: usize = 40;
    let short: String = text.chars().take(EXCERPT_CHARS).collect();
    if short.len() < text.len() {
        format!("{short}…")
    } else {
        short
    }
}

pub(super) fn invalid(detail: String) -> Error {
    Error::Invalid(format!("Couldn't read a palette: {detail}."))
}

#[cfg(test)]
mod tests;
