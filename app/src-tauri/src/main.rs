//! Tracewright desktop app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tracewright_app::run();
}
