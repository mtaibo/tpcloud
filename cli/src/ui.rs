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

/// A choice made with one key, no Enter: `prompt [default]: ` then the key pressed (Enter takes
/// the default). Not a terminal (or more than nine options): a typed line, as before.
pub fn key(prompt: &str, count: usize, default: usize) -> anyhow::Result<usize> {
    use std::io::{Read, Write};
    let default = default.min(count.saturating_sub(1));
    print!("{prompt} {}: ", dim(&format!("[{}]", default + 1)));
    std::io::stdout().flush()?;
    let mut term: libc::termios = unsafe { std::mem::zeroed() };
    let tty = std::io::stdin().is_terminal() && count <= 9 && unsafe { libc::tcgetattr(0, &mut term) } == 0;
    if !tty {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line)? == 0 {
            anyhow::bail!("no answer");
        }
        let line = line.trim();
        return Ok(if line.is_empty() { default } else { line.parse::<usize>().ok().filter(|n| (1..=count).contains(n)).map_or(default, |n| n - 1) });
    }
    let mut raw = term;
    raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
    raw.c_cc[libc::VMIN] = 1;
    raw.c_cc[libc::VTIME] = 0;
    unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) };
    let restore = || unsafe { libc::tcsetattr(0, libc::TCSANOW, &term) };
    let mut byte = [0u8; 1];
    let picked = loop {
        if std::io::stdin().read(&mut byte).unwrap_or(0) == 0 {
            restore();
            anyhow::bail!("no answer");
        }
        match byte[0] {
            b'\r' | b'\n' => break default,
            3 | 4 => {
                // Ctrl-C, Ctrl-D: out, with the terminal as it was.
                restore();
                println!();
                std::process::exit(130);
            }
            b @ b'1'..=b'9' if ((b - b'0') as usize) <= count => break (b - b'1') as usize,
            _ => {}
        }
    };
    restore();
    println!("{}", accent(&(picked + 1).to_string()));
    Ok(picked)
}
