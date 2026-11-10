//! `fs-history`: the one command of the program (ADR 0063). Without words it opens the window.

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    if let Err(message) = fs_history_app::gui::run(&words) {
        eprintln!("fs-history: {message}");
        std::process::exit(2);
    }
}
