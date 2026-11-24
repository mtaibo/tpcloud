//! `tpcloud --help`: the commands in groups, with colour. Each command's own `--help` (clap's,
//! styled to match) has the details.

use crate::ui;

type Row = (&'static str, &'static str, &'static str); // command, its arguments, what it does

const NETWORK: &[Row] = &[
    ("login", "[address]", "Sign in, to the one found on this Wi-Fi or at an address"),
    ("logout", "", "Sign out of the network in use"),
    ("whoami", "", "Who you're signed in as, and where"),
    ("networks", "[use|remove <name>]", "The networks this computer knows"),
];

const CLOUD: &[Row] = &[
    ("sync", "[add|keep|free|mode]", "Your files in a folder here, on demand or copied"),
    ("home", "", "The house: blinds and switches (also `tphome`)"),
    ("sessions", "[revoke <id>]", "Where you're signed in; close one"),
];

const ADMINS: &[Row] = &[
    ("users", "[add|remove|admin]", "The accounts"),
    ("sessions", "--all", "Everyone's sessions"),
    ("host", "[check|update]", "The server's version, and updating it"),
];

const THIS_COMPUTER: &[Row] = &[
    ("update", "", "The latest tpcloud, and the app"),
    ("delete", "", "Every trace of TPCloud, gone"),
];

#[cfg(feature = "host")]
const SERVER: &[Row] = &[
    ("status", "", "What's running, and where it's reached"),
    ("setup", "", "The guided configuration (sudo)"),
    ("start", "", "Start everything"),
    ("stop", "", "Stop everything; data is kept"),
    ("restart", "", "Stop and start again"),
    ("logs", "[service] [-f]", "What a process is saying"),
    ("module", "<enable|disable>", "Turn Store or Home on and off"),
    ("storage", "", "The disks and the storage pool"),
    ("backup", "[--list|--to|--off]", "Back up settings and databases now"),
    ("restore", "<backup>", "Put a backup back"),
    ("update", "", "The latest release, for the whole server (sudo)"),
    ("delete", "", "Asks what goes: the program, or everything (sudo)"),
];

fn section(title: &str, rows: &[Row]) {
    println!("  {}", ui::bold(title));
    for (cmd, args, what) in rows {
        let left = if args.is_empty() { cmd.to_string() } else { format!("{cmd} {args}") };
        let pad = 30usize.saturating_sub(left.chars().count());
        let args = if args.is_empty() { String::new() } else { format!(" {}", ui::dim(args)) };
        println!("    {}{args}{}{what}", ui::accent(cmd), " ".repeat(pad));
    }
    println!();
}

pub fn print(server: bool) {
    println!();
    println!("  {} {}", ui::bold(&ui::accent("TPCloud")), ui::dim(&format!("v{}", env!("CARGO_PKG_VERSION"))));
    println!("  {}", ui::dim("Your cloud, from the terminal."));
    println!();
    println!("  {} {} {}", ui::green("›"), ui::accent("tpcloud <command> --help"), ui::dim("tells you everything about one"));
    println!();
    println!("  {}{}", ui::accent("tpcloud"), "                         Alone: everything in one screen");
    println!();
    #[cfg(feature = "host")]
    if server {
        section("This server", SERVER);
    }
    section("Your network", NETWORK);
    section("Your cloud", CLOUD);
    section("For admins", ADMINS);
    if !server {
        section("This computer", THIS_COMPUTER);
    }
}

/// Parses the arguments; a command's own `--help` comes out like ours (two spaces in, with a
/// blank line around it), errors as clap prints them.
pub fn parse<T: clap::Parser>(args: impl IntoIterator<Item = String>) -> T {
    use clap::error::ErrorKind;
    use std::io::IsTerminal;
    match T::try_parse_from(args) {
        Ok(parsed) => parsed,
        Err(err) if matches!(err.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand) => {
            let tty = std::io::stdout().is_terminal();
            let text = if tty { err.render().ansi().to_string() } else { err.render().to_string() };
            // The command's line on top in bold, like a title.
            let (first, rest) = text.split_once('\n').unwrap_or((&text, ""));
            print!("{}", ui::indented(&format!("{}\n{rest}", ui::bold(first))));
            println!();
            std::process::exit(0);
        }
        Err(err) => err.exit(),
    }
}

/// The same look for clap's own help and errors.
pub fn styles() -> clap::builder::Styles {
    use clap::builder::styling::{Ansi256Color, Color, Style};
    let accent = Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(75))));
    clap::builder::Styles::styled()
        .header(accent.bold())
        .usage(accent.bold())
        .literal(accent)
        .placeholder(Style::new().dimmed())
        .valid(accent)
        .invalid(Style::new().fg_color(Some(Color::Ansi256(Ansi256Color(215)))))
        .error(Style::new().bold().fg_color(Some(Color::Ansi256(Ansi256Color(215)))))
}
