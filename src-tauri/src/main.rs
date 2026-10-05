// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use winotch_lib::modules::claude_code::ClaudeCode;

fn main() {
    winotch_lib::run(vec![Box::new(ClaudeCode::default())]);
}
