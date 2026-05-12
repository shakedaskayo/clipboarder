#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if clipboarder_lib::cli::looks_like_cli(&args) {
        if let Err(e) = clipboarder_lib::cli::run() {
            eprintln!("clipboarder: {e:#}");
            std::process::exit(3);
        }
        return;
    }
    clipboarder_lib::run();
}
