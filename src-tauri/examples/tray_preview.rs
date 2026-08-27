//! Render a few tray icon previews to disk for visual QA.
//!
//! Run with: `cargo run --example tray_preview`

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We cannot easily import the binary crate's modules in an example, so
    // we shell out to a quick render via the public function in the lib.
    // The example is meant for local QA only and is not part of the shipped
    // app, so reaching into the lib is fine.
    let lib = codex_runway_windows_lib::tray_icon::render_png;
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("docs/tray-preview");
    std::fs::create_dir_all(&out_dir)?;
    for pct in [0u32, 25, 50, 70, 89, 90, 100] {
        let bytes = lib(pct as f32);
        let path = out_dir.join(format!("tray-{pct:03}.png"));
        std::fs::write(&path, &bytes)?;
        println!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
    Ok(())
}
