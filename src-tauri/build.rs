use tauri_build::{Attributes, WindowsAttributes};

fn main() {
    // Set when another app builds the core as a library (e.g. an assembled
    // build, through its .cargo/config.toml): that app's own tauri-build links
    // the C runtime statically. Doing it here too would put an empty
    // msvcrt.lib on the link path of every crate depending on the core, and
    // their test binaries would fail to link.
    println!("cargo:rerun-if-env-changed=MINIM_NOTCH_EMBEDDED");
    let embedded = std::env::var_os("MINIM_NOTCH_EMBEDDED").is_some();
    let windows = WindowsAttributes::new().static_vc_runtime(!embedded);
    tauri_build::try_build(Attributes::new().windows_attributes(windows))
        .expect("failed to run tauri-build");
}
