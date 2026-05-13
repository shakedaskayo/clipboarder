#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Default SIGPIPE handling: terminate cleanly instead of Rust's stdout
    // panic when our CLI output is piped into `head`, `cb p | pbcopy`, etc.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

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
