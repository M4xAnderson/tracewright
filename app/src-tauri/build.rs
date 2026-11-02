//! Registers the app's commands through the app manifest (SEC-1), so a capability must name each one.

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "ui_ready",
        "pick_project_folder",
        "open_project",
        "run_checks",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("tauri build script failed; see the message above");
}
