//! GNU Emacs numeric color syntax, shared by faces and image decoders.

/// GNU's `(len - 1) % 3 == 0` arm of `parse_color_spec` (src/xfaces.c:984).
fn parse_hex_color_payload(payload: &[u8]) -> Option<(i64, i64, i64)> {
    if payload.is_empty() || !payload.len().is_multiple_of(3) {
        return None;
    }
    let component_len = payload.len() / 3;
    let red = parse_hex_color_comp(&payload[..component_len])?;
    let green = parse_hex_color_comp(&payload[component_len..2 * component_len])?;
    let blue = parse_hex_color_comp(&payload[2 * component_len..])?;
    Some((i64::from(red), i64::from(green), i64::from(blue)))
}

/// One hex color component of 1-4 digits, normalized so the maximum value for
/// that digit count becomes 65535.
///
/// Mirrors GNU `parse_hex_color_comp` (src/xfaces.c:928): it walks the spec's
/// BYTES and fails on any non-hex byte, so a multi-byte character (whose UTF-8
/// bytes are all non-hex) is rejected rather than sliced.
fn parse_hex_color_comp(component: &[u8]) -> Option<u16> {
    let digits = component.len();
    if digits == 0 || digits > 4 {
        return None;
    }
    let mut value: u32 = 0;
    for &byte in component {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'A'..=b'F' => byte - b'A' + 10,
            b'a'..=b'f' => byte - b'a' + 10,
            _ => return None,
        };
        value = (value << 4) | u32::from(digit);
    }
    let max_value = (1u32 << (digits * 4)) - 1;
    Some((value * 65535 / max_value) as u16)
}

/// Decimal float component in [0,1], scaled to 16 bits.
///
/// Mirrors GNU `parse_float_color_comp` (src/xfaces.c:955): only decimal
/// literals without whitespace are accepted; an EMPTY component is `strtod`'s
/// 0.0 with `end == s == e`, so it parses as 0; and the scale uses `lrint`'s
/// round-half-to-even.
fn parse_float_color_comp(component: &[u8]) -> Option<u16> {
    if !component
        .iter()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'.' | b'+' | b'-' | b'e' | b'E'))
    {
        return None;
    }
    let value: f64 = if component.is_empty() {
        0.0
    } else {
        // Every accepted byte is ASCII, so the UTF-8 conversion cannot fail,
        // and `parse` consumes the whole component like GNU's `end == e`.
        std::str::from_utf8(component).ok()?.parse().ok()?
    };
    if (0.0..=1.0).contains(&value) {
        Some((value * 65535.0).round_ties_even() as u16)
    } else {
        None
    }
}

/// GNU `parse_color_spec` (src/xfaces.c:976): the three numeric color forms
/// `#RGB`, `rgb:R/G/B` and `rgbi:R/G/B`, each component 1-4 hex digits (or a
/// float in [0,1] for `rgbi`).
pub fn parse_color_spec(spec: &[u8]) -> Option<(i64, i64, i64)> {
    if let Some(payload) = spec.strip_prefix(b"#") {
        return parse_hex_color_payload(payload);
    }
    if let Some(rest) = spec.strip_prefix(b"rgb:") {
        let mut components = rest.splitn(3, |&byte| byte == b'/');
        let red = parse_hex_color_comp(components.next()?)?;
        let green = parse_hex_color_comp(components.next()?)?;
        // GNU measures the last component to the end of the string, so a
        // further '/' stays inside it and fails the hex validation.
        let blue = parse_hex_color_comp(components.next()?)?;
        return Some((i64::from(red), i64::from(green), i64::from(blue)));
    }
    if let Some(rest) = spec.strip_prefix(b"rgbi:") {
        let mut components = rest.splitn(3, |&byte| byte == b'/');
        let red = parse_float_color_comp(components.next()?)?;
        let green = parse_float_color_comp(components.next()?)?;
        let blue = parse_float_color_comp(components.next()?)?;
        return Some((i64::from(red), i64::from(green), i64::from(blue)));
    }
    None
}

/// Resolve a GUI color to the renderer's eight-bit sRGB channels.
/// Numeric components are normalized to 16 bits before taking the high byte.
pub fn resolve_color(spec: &str) -> Option<(u8, u8, u8)> {
    parse_color_spec(spec.as_bytes())
        .map(|(r, g, b)| ((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8))
        .or_else(|| crate::x11_colors::x11_color_lookup(spec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gnu_numeric_normalization_and_x11_names() {
        for (spec, expected) in [
            ("#123", (0x1111, 0x2222, 0x3333)),
            ("#123456", (0x1212, 0x3434, 0x5656)),
            ("#123456789", (4657, 17764, 30871)),
            ("#123456789abc", (0x1234, 0x5678, 0x9abc)),
            ("rgb:1/22/333", (0x1111, 0x2222, 0x3333)),
            ("rgbi:0.5/0/1", (32768, 0, 65535)),
            ("rgbi://", (0, 0, 0)),
        ] {
            assert_eq!(parse_color_spec(spec.as_bytes()), Some(expected), "{spec}");
        }
        assert_eq!(resolve_color("green"), Some((0, 255, 0)));
        assert_eq!(
            resolve_color("Light Goldenrod Yellow"),
            Some((250, 250, 210))
        );
        for spec in [
            "#あ",
            "#12é45",
            "#ggg",
            "#1234",
            "rgb:1/2/3/4",
            "rgbi:NaN/0/0",
        ] {
            assert_eq!(resolve_color(spec), None, "{spec}");
        }
    }
}
