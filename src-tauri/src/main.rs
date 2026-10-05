// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use winotch_lib::modules::claude_code::ClaudeCode;
use winotch_lib::modules::clock::Clock;

fn main() {
    // Display order: at equal urgency, the first module's items come first.
    winotch_lib::run(
        tauri::generate_context!(),
        vec![Box::new(ClaudeCode::default()), Box::new(Clock::default())],
    );
}
