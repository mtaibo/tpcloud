//! Terminal output: colour only when it's a terminal.

use std::io::IsTerminal;

fn paint(code: &str, text: &str) -> String {
    if std::io::stdout().is_terminal() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn green(text: &str) -> String {
    paint("38;5;114", text)
}
pub fn dim(text: &str) -> String {
    paint("2", text)
}
pub fn bold(text: &str) -> String {
    paint("1", text)
}
pub fn accent(text: &str) -> String {
    paint("38;5;75", text)
}

pub fn step(text: &str) {
    println!("{} {text}", accent("›"));
}
pub fn done(text: &str) {
    println!("{} {text}", green("✓"));
}
pub fn warn(text: &str) {
    eprintln!("{} {text}", paint("38;5;215", "!"));
}

pub fn warn_dot() -> String {
    paint("38;5;215", "●")
}
