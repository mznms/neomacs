use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let project_root = PathBuf::from(
        std::env::var_os("CARGO_WORKSPACE_DIR")
            .expect("workspace .cargo/config.toml must set CARGO_WORKSPACE_DIR"),
    );
    generate_x11_color_table(&project_root);
}

/// Parse etc/rgb.txt and generate a Rust source file with a static
/// color lookup function shared by faces and images, including numbered
/// grays and the compatibility names documented in the database. The table
/// is compiled into the binary with zero runtime file I/O.
fn generate_x11_color_table(project_root: &Path) {
    let rgb_path = project_root.join("etc/rgb.txt");
    println!("cargo:rerun-if-changed={}", rgb_path.display());

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let out_path = out_dir.join("x11_colors.rs");

    let content = fs::read_to_string(&rgb_path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {e}", rgb_path.display()));

    // Parse rgb.txt: "R G B\t\tColorName"
    // Collect unique (lowercase_name -> (r, g, b)), also add no-space variants.
    let mut colors: std::collections::BTreeMap<String, (u8, u8, u8)> =
        std::collections::BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let r = parts.next().and_then(|s| s.parse::<u8>().ok());
        let g = parts.next().and_then(|s| s.parse::<u8>().ok());
        let b = parts.next().and_then(|s| s.parse::<u8>().ok());
        // Remaining is the color name (may contain spaces)
        let name: String = parts.collect::<Vec<_>>().join(" ");

        if let (Some(r), Some(g), Some(b)) = (r, g, b)
            && !name.is_empty()
        {
            let lower = name.to_lowercase();
            let no_spaces = lower.replace(' ', "");
            colors.entry(lower.clone()).or_insert((r, g, b));
            if no_spaces != lower {
                colors.entry(no_spaces).or_insert((r, g, b));
            }
        }
    }

    // Generate Rust source: a function with a match statement.
    let mut code = String::new();
    code.push_str("/// Auto-generated from etc/rgb.txt — do not edit.\n");
    code.push_str("/// X11 color name lookup (case-insensitive).\n");
    code.push_str("pub fn x11_color_lookup(name: &str) -> Option<(u8, u8, u8)> {\n");
    code.push_str("    match name.to_lowercase().as_str() {\n");
    for (name, (r, g, b)) in &colors {
        code.push_str(&format!(
            "        {:?} => Some(({}, {}, {})),\n",
            name, r, g, b
        ));
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(&out_path, &code).expect("Failed to write x11_colors.rs");
    eprintln!(
        "cargo:warning=Generated X11 color table: {} entries from {}",
        colors.len(),
        rgb_path.display()
    );
}
