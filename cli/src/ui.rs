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

pub fn warn_text(text: &str) -> String {
    paint("38;5;215", text)
}

#[cfg_attr(not(feature = "host"), allow(dead_code))]
pub fn warn_dot() -> String {
    paint("38;5;215", "●")
}

/// A line typed without showing it (passwords).
pub fn secret(question: &str) -> anyhow::Result<String> {
    use std::io::{BufRead, Write};
    print!("{question}: ");
    std::io::stdout().flush()?;
    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    let tty = unsafe { libc::tcgetattr(0, &mut term) } == 0;
    if tty {
        let mut silent = term;
        silent.c_lflag &= !libc::ECHO;
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &silent) };
    }
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    if tty {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &term) };
    }
    println!();
    Ok(line.trim().to_string())
}
