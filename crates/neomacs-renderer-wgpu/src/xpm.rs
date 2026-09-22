//! Pure Rust XPM (X PixMap) decoder
//!
//! Parses XPM2 and XPM3 format image data and produces RGBA pixel buffers.

use std::collections::HashMap;
use std::path::Path;

use neomacs_display_protocol::{ImageColorContext, color_spec::resolve_color};

/// Decode XPM image from in-memory data, returning (width, height, rgba_pixels).
pub fn decode_xpm_data(data: &[u8], context: &ImageColorContext) -> Option<(u32, u32, Vec<u8>)> {
    let strings = extract_strings(data)?;
    decode_from_strings(&strings, context)
}

/// Decode XPM image from a file path.
pub fn decode_xpm_file(path: &Path, context: &ImageColorContext) -> Option<(u32, u32, Vec<u8>)> {
    let data = std::fs::read(path).ok()?;
    decode_xpm_data(&data, context)
}

/// Query XPM dimensions without full decode (header only).
pub fn query_xpm_dimensions(data: &[u8]) -> Option<(u32, u32)> {
    let strings = extract_strings(data)?;
    if strings.is_empty() {
        return None;
    }
    let header = parse_header(strings[0])?;
    Some((header.width, header.height))
}

struct XpmHeader {
    width: u32,
    height: u32,
    ncolors: u32,
    chars_per_pixel: u32,
}

fn parse_header(s: &[u8]) -> Option<XpmHeader> {
    let text = std::str::from_utf8(s).ok()?;
    let mut parts = text.split_whitespace();
    let width: u32 = parts.next()?.parse().ok()?;
    let height: u32 = parts.next()?.parse().ok()?;
    let ncolors: u32 = parts.next()?.parse().ok()?;
    let chars_per_pixel: u32 = parts.next()?.parse().ok()?;
    if width == 0 || height == 0 || ncolors == 0 || chars_per_pixel == 0 {
        return None;
    }
    Some(XpmHeader {
        width,
        height,
        ncolors,
        chars_per_pixel,
    })
}

/// Extract quoted strings from XPM data.
/// Handles both XPM3 (/* XPM */ with C string array) and XPM2 (! XPM2 with plain lines).
fn extract_strings(data: &[u8]) -> Option<Vec<&[u8]>> {
    // Check for XPM2 format: starts with "! XPM2"
    if data.starts_with(b"! XPM2") {
        return extract_xpm2_lines(data);
    }

    // XPM3 format: extract C string literals between double quotes
    let mut strings = Vec::new();
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'"' {
            i += 1;
            let start = i;
            while i < data.len() && data[i] != b'"' {
                // Handle backslash escapes
                if data[i] == b'\\' && i + 1 < data.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            strings.push(&data[start..i]);
            if i < data.len() {
                i += 1; // skip closing quote
            }
        } else {
            i += 1;
        }
    }

    if strings.is_empty() {
        None
    } else {
        Some(strings)
    }
}

/// Extract lines from XPM2 format (plain text, no C wrapper).
fn extract_xpm2_lines(data: &[u8]) -> Option<Vec<&[u8]>> {
    let mut lines: Vec<&[u8]> = Vec::new();
    for line in data.split(|&b| b == b'\n') {
        let trimmed = trim_bytes(line);
        // Skip empty lines and the header line "! XPM2"
        if trimmed.is_empty() || trimmed.starts_with(b"! XPM2") || trimmed.starts_with(b"!") {
            continue;
        }
        lines.push(trimmed);
    }
    if lines.is_empty() { None } else { Some(lines) }
}

fn trim_bytes(b: &[u8]) -> &[u8] {
    let start = b
        .iter()
        .position(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .unwrap_or(b.len());
    let end = b
        .iter()
        .rposition(|&c| c != b' ' && c != b'\t' && c != b'\r')
        .map_or(start, |p| p + 1);
    &b[start..end]
}

fn decode_from_strings(
    strings: &[&[u8]],
    context: &ImageColorContext,
) -> Option<(u32, u32, Vec<u8>)> {
    if strings.is_empty() {
        return None;
    }

    let header = parse_header(strings[0])?;
    let cpp = header.chars_per_pixel as usize;
    let expected_strings = 1 + header.ncolors as usize + header.height as usize;
    if strings.len() < expected_strings {
        tracing::warn!(
            "XPM: expected {} strings, got {}",
            expected_strings,
            strings.len()
        );
        return None;
    }

    // Parse color table
    let mut colors: HashMap<Vec<u8>, [u8; 4]> = HashMap::with_capacity(header.ncolors as usize);
    for i in 0..header.ncolors as usize {
        let line = strings[1 + i];
        if line.len() < cpp {
            tracing::warn!("XPM: color line {} too short", i);
            return None;
        }
        let key = line[..cpp].to_vec();
        let rest = &line[cpp..];
        let color = parse_color_def(rest, context)?;
        colors.insert(key, color);
    }

    // Parse pixel data
    let w = header.width as usize;
    let h = header.height as usize;
    let mut rgba = vec![0u8; w * h * 4];

    let fallback = context.frame_foreground().rgba8();
    for y in 0..h {
        let row = strings[1 + header.ncolors as usize + y];
        for x in 0..w {
            let start = x * cpp;
            let end = start + cpp;
            if end > row.len() {
                tracing::warn!(
                    "XPM: row {} too short (need {} bytes, have {})",
                    y,
                    end,
                    row.len()
                );
                return None;
            }
            let pixel_key = &row[start..end];
            let color = colors.get(pixel_key).unwrap_or(&fallback);
            let idx = (y * w + x) * 4;
            rgba[idx] = color[0];
            rgba[idx + 1] = color[1];
            rgba[idx + 2] = color[2];
            rgba[idx + 3] = color[3];
        }
    }

    let width = header.width;
    let height = header.height;

    Some((width, height, rgba))
}

/// Parse a color definition from the rest of a color line (after the pixel key).
/// Looks for "c <color>" (visual color key). Falls back to other keys.
fn parse_color_def(rest: &[u8], context: &ImageColorContext) -> Option<[u8; 4]> {
    let text = std::str::from_utf8(rest).ok()?;
    let tokens: Vec<&str> = text.split_whitespace().collect();

    // Each value extends to the next key, consuming at least one token so
    // symbolic names such as "c" remain valid. Prefer c, g, g4, then m.
    let keys = ["c", "g", "g4", "m", "s"];
    let mut best = None;
    let mut symbol = None;
    let mut index = 0;
    while index + 1 < tokens.len() {
        let priority = keys.iter().position(|key| tokens[index] == *key);
        let start = index + 1;
        let end = (start + 1..tokens.len())
            .find(|&i| keys.contains(&tokens[i]))
            .unwrap_or(tokens.len());
        if let Some(priority @ 0..=3) = priority
            && best.as_ref().is_none_or(|(rank, _)| priority < *rank)
        {
            best = Some((priority, tokens[start..end].join(" ")));
        } else if priority == Some(4) {
            symbol = Some(tokens[start..end].join(" "));
        }
        index = end;
    }

    // GNU uses assoc (case-sensitive, first matching entry). An invalid
    // override falls back to the selected visual color, including None.
    let replacement = symbol.as_ref().and_then(|symbol| {
        context
            .xpm_color_symbols()
            .iter()
            .find(|(name, _)| name == symbol)
            .and_then(|(_, color)| parse_color_value(color))
    });
    Some(
        replacement
            .or_else(|| best.and_then(|(_, color)| parse_color_value(&color)))
            .unwrap_or_else(|| context.frame_foreground().rgba8()),
    )
}

/// Transparency is XPM syntax; all other colors use the face color resolver.
fn parse_color_value(s: &str) -> Option<[u8; 4]> {
    if s.eq_ignore_ascii_case("none") {
        return Some([0, 0, 0, 0]);
    }
    resolve_color(s).map(|(r, g, b)| [r, g, b, 255])
}

#[cfg(test)]
#[path = "xpm_test.rs"]
mod tests;
