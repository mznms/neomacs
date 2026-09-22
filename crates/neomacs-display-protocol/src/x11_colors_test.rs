use super::*;

#[test]
fn additional_xorg_colors_are_available_to_all_consumers() {
    for (name, expected) in [
        ("aqua", (0, 255, 255)),
        ("fuchsia", (255, 0, 255)),
        ("olive", (128, 128, 0)),
        ("teal", (0, 128, 128)),
        ("silver", (192, 192, 192)),
        ("crimson", (220, 20, 60)),
        ("indigo", (75, 0, 130)),
        ("lime", (0, 255, 0)),
    ] {
        assert_eq!(x11_color_lookup(name), Some(expected), "{name}");
        assert_eq!(x11_color_lookup(&name.to_uppercase()), Some(expected));
    }
}

#[test]
fn legacy_darkyellow_alias_is_shared_without_accepting_unknown_names() {
    assert_eq!(x11_color_lookup("darkyellow"), Some((189, 183, 107)));
    assert_eq!(
        x11_color_lookup("DarkYellow"),
        x11_color_lookup("darkkhaki")
    );
    assert_eq!(x11_color_lookup("unknown-color"), None);
}
