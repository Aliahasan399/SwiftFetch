// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if !swiftfetch_lib::run_update_helper_if_requested() && !swiftfetch_lib::run_native_host_if_requested() {
        swiftfetch_lib::run();
    }
}
