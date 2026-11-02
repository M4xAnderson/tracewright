//! The commands the UI calls and the types they exchange. Bindings come from ts-rs (IPC-2); the
//! tests write JSON fixtures that the UI tests replay, so both sides agree on the wire format.

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;

use crate::log;

/// A screen the UI can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "screen.ts")]
pub enum Screen {
    Start,
}

/// Sent once the UI has rendered its first screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "ui-ready.ts")]
pub struct UiReady {
    pub screen: Screen,
}

/// What the app does once the UI is ready; chosen at start from `TRACEWRIGHT_SMOKE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Smoke {
    Off,
    /// The real-window smoke test: exit 0 once the start screen is shown.
    ExitWhenStartScreenShown,
}

/// The UI rendered its first screen; logged, so a blank window is visible in the log.
#[tauri::command]
pub async fn ui_ready(app: AppHandle, ready: UiReady) {
    log::line(
        log::Level::Info,
        &format!("UI ready: {:?} screen shown", ready.screen),
    );
    let smoke = *app.state::<Smoke>();
    if smoke == Smoke::ExitWhenStartScreenShown && ready.screen == Screen::Start {
        log::line(log::Level::Info, "smoke test passed; exiting");
        app.exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    #[expect(
        clippy::disallowed_methods,
        reason = "writes a test fixture for the UI, not a project file"
    )]
    fn ui_ready_fixture() {
        let ready = UiReady {
            screen: Screen::Start,
        };
        let json = serde_json::to_string_pretty(&ready).expect("UiReady serializes");
        assert_eq!(
            json, "{\n  \"screen\": \"start\"\n}",
            "the UI sends the screen in camelCase"
        );

        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../ui/src/ipc/fixtures/ui-ready.json");
        std::fs::write(&fixture, format!("{json}\n")).expect("fixture written");
        let parsed: UiReady = serde_json::from_str(&json).expect("UiReady deserializes");
        assert_eq!(parsed, ready, "the fixture round-trips");
    }
}
