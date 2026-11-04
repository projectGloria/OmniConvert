fn main() {
    // Generates the Windows resource (icon, manifest) and validates tauri.conf.json
    // + capabilities at compile time — a typo'd permission fails `cargo check`, not runtime.
    tauri_build::build()
}
