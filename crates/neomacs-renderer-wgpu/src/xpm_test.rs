use super::*;

#[test]
fn test_basic_xpm3() {
    let xpm = br#"/* XPM */
static char * test[] = {
"4 4 2 1",
"  c None",
"X c #FF0000",
"XXXX",
"X  X",
"X  X",
"XXXX"
};"#;
    let result = decode_xpm_data(xpm);
    assert!(result.is_some());
    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 4);
    assert_eq!(h, 4);
    assert_eq!(rgba.len(), 64); // 4*4*4
    // Top-left pixel should be red
    assert_eq!(&rgba[0..4], &[255, 0, 0, 255]);
    // Second pixel in second row should be transparent
    assert_eq!(&rgba[(4 + 1) * 4..(4 + 1) * 4 + 4], &[0, 0, 0, 0]);
}

#[test]
fn test_query_dimensions() {
    let xpm = br#"/* XPM */
static char * test[] = {
"10 20 2 1",
"  c None",
"X c #000000",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX",
"XXXXXXXXXX"
};"#;
    let dims = query_xpm_dimensions(xpm);
    assert_eq!(dims, Some((10, 20)));
}

#[test]
fn test_hex_colors() {
    assert_eq!(parse_hex_color("FF0000"), [255, 0, 0, 255]);
    assert_eq!(parse_hex_color("00FF00"), [0, 255, 0, 255]);
    assert_eq!(parse_hex_color("0000FF"), [0, 0, 255, 255]);
    assert_eq!(parse_hex_color("F00"), [255, 0, 0, 255]);
    assert_eq!(parse_hex_color("FFFF00000000"), [255, 0, 0, 255]);
}

#[test]
fn test_named_colors() {
    assert_eq!(parse_color_value("None"), [0, 0, 0, 0]);
    assert_eq!(parse_color_value("white"), [255, 255, 255, 255]);
    assert_eq!(parse_color_value("black"), [0, 0, 0, 255]);
    assert_eq!(parse_color_value("red"), [255, 0, 0, 255]);
}

#[test]
fn test_nyan_grayscale_palette() {
    // Nyan mode uses these X11 names in its XPM palette. In particular,
    // gray50 is 127 in rgb.txt, not the 128 obtained by rounding 255 * 0.5.
    let xpm = br#"/* XPM */
static char * test[] = {
"8 1 8 1",
"a c gray0",
"b c gray15",
"c c gray50",
"d c gray60",
"e c gray81",
"f c GrEy60",
"g c gray100",
"h c None",
"abcdefgh"
};"#;
    let (width, height, rgba) = decode_xpm_data(xpm).unwrap();
    assert_eq!((width, height), (8, 1));
    assert_eq!(
        rgba,
        [
            [0, 0, 0, 255],
            [38, 38, 38, 255],
            [127, 127, 127, 255],
            [153, 153, 153, 255],
            [207, 207, 207, 255],
            [153, 153, 153, 255],
            [255, 255, 255, 255],
            [0, 0, 0, 0],
        ]
        .concat()
    );
}

#[test]
fn test_named_colors_use_x11_values() {
    for (name, expected) in [
        ("green", [0, 255, 0, 255]),
        ("gray", [190, 190, 190, 255]),
        ("purple", [160, 32, 240, 255]),
        ("maroon", [176, 48, 96, 255]),
        ("DarkGoldenrod3", [205, 149, 12, 255]),
    ] {
        assert_eq!(parse_color_value(name), expected, "{name}");
    }
}

#[test]
fn test_additional_named_colors() {
    for (name, expected) in [
        ("aqua", [0, 255, 255, 255]),
        ("fuchsia", [255, 0, 255, 255]),
        ("olive", [128, 128, 0, 255]),
        ("teal", [0, 128, 128, 255]),
        ("silver", [192, 192, 192, 255]),
        ("crimson", [220, 20, 60, 255]),
        ("indigo", [75, 0, 130, 255]),
        ("lime", [0, 255, 0, 255]),
        ("darkyellow", [189, 183, 107, 255]),
        ("unknown-color", [0, 0, 0, 255]),
    ] {
        assert_eq!(parse_color_value(name), expected, "{name}");
    }
}

#[test]
fn test_x11_palette_through_decoder() {
    // Exercise every spelling in GNU's database, including gray0..gray100,
    // grey0..grey100, numbered colors, and names containing spaces.
    let database = include_str!(concat!(env!("CARGO_WORKSPACE_DIR"), "/etc/rgb.txt"));
    for line in database.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', '!']) {
            continue;
        }
        let mut fields = line.split_whitespace();
        let mut expected = [0, 0, 0, 255];
        for channel in &mut expected[..3] {
            *channel = fields.next().unwrap().parse::<u8>().unwrap();
        }
        let name = fields.collect::<Vec<_>>().join(" ");
        let xpm = format!("! XPM2\n1 1 1 1\nx c {name}\nx\n");
        let (width, height, rgba) = decode_xpm_data(xpm.as_bytes()).unwrap();
        assert_eq!((width, height), (1, 1), "{name}");
        assert_eq!(rgba, expected, "{name}");
    }
}

#[test]
fn test_color_fields_preserve_names_and_visual_precedence() {
    for definition in [
        "s accent m black g4 gray50 g gray60 c light goldenrod yellow",
        "c light goldenrod yellow s accent g gray60 m black",
        "g gray60 c LightGoldenrodYellow g4 gray50",
    ] {
        assert_eq!(
            parse_color_def(definition.as_bytes()),
            Some([250, 250, 210, 255])
        );
    }
    // A symbolic name may itself be a key word; it is not a visual color.
    assert_eq!(parse_color_def(b"s c c gray60"), Some([153, 153, 153, 255]));
    assert_eq!(
        parse_color_def(b"m black g4 gray50 g gray60"),
        Some([153, 153, 153, 255])
    );
    assert_eq!(
        parse_color_def(b"m black g4 gray50"),
        Some([127, 127, 127, 255])
    );
    assert_eq!(parse_color_def(b"m white"), Some([255, 255, 255, 255]));
}

#[test]
fn test_multi_cpp() {
    // chars_per_pixel = 2
    let xpm = br###"/* XPM */
static char * test[] = {
"2 2 3 2",
".. c #FFFFFF",
"## c #000000",
"   c None",
"..##",
"##.."
};"###;
    let result = decode_xpm_data(xpm);
    assert!(result.is_some());
    let (w, h, rgba) = result.unwrap();
    assert_eq!(w, 2);
    assert_eq!(h, 2);
    // (0,0) = white
    assert_eq!(&rgba[0..4], &[255, 255, 255, 255]);
    // (1,0) = black
    assert_eq!(&rgba[4..8], &[0, 0, 0, 255]);
    // (0,1) = black
    assert_eq!(&rgba[8..12], &[0, 0, 0, 255]);
    // (1,1) = white
    assert_eq!(&rgba[12..16], &[255, 255, 255, 255]);
}
