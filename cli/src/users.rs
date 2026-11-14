//! Accounts, from the server itself: auth's own scripts run with the bundled Python, straight on
//! its database (whoever can run them here already owns the server).

use crate::{paths, ui};
use anyhow::{bail, Context, Result};
use std::process::{Command, Output};

fn python(script: &str, args: &[&str], extra_env: &[(&str, &str)]) -> Result<Output> {
    let rt = paths::runtime_dir();
    let code = rt.join("code/auth");
    let db = paths::state_dir().join("data/auth/tpauth.db");
    std::fs::create_dir_all(db.parent().unwrap())?;
    let mut cmd = Command::new(rt.join("python/bin/python3"));
    cmd.arg(code.join("scripts").join(script))
        .args(args)
        .current_dir(&code)
        .env_clear()
        .env("PYTHONPATH", format!("{}:{}", rt.join("py/auth").display(), code.display()))
        .env("PYTHONDONTWRITEBYTECODE", "1")
        .env("DATABASE_URL", format!("sqlite:///{}", db.display()))
        .env("TPCLOUD_URL", "http://localhost");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output().context("couldn't run the bundled Python (is TPCloud installed?)")
}

pub struct User {
    pub email: String,
    pub admin: bool,
    pub totp: bool,
}

pub fn list() -> Result<Vec<User>> {
    let out = python("manage.py", &["list"], &[])?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .map(|u| User {
            email: u["email"].as_str().unwrap_or("").to_string(),
            admin: u["admin"].as_bool().unwrap_or(false),
            totp: u["totp"].as_bool().unwrap_or(false),
        })
        .collect())
}

pub fn count() -> Result<usize> {
    if !paths::state_dir().join("data/auth/tpauth.db").exists() {
        return Ok(0);
    }
    Ok(list()?.len())
}

pub fn add(email: &str, password: &str, admin: bool) -> Result<()> {
    let mut args = vec!["add", email];
    if admin {
        args.push("--admin");
    }
    let out = python("manage.py", &args, &[("NEW_PASSWORD", password)])?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    ui::done(String::from_utf8_lossy(&out.stdout).trim());
    Ok(())
}

pub fn create_owner(email: &str, password: &str) -> Result<()> {
    let out = python("bootstrap.py", &[], &[("OWNER_EMAIL", email), ("OWNER_PASSWORD", password)])?;
    if !out.status.success() {
        bail!("creating your account failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    ui::done(String::from_utf8_lossy(&out.stdout).trim());
    Ok(())
}

pub fn print_list() -> Result<()> {
    let users = list()?;
    if users.is_empty() {
        println!("No accounts yet: `sudo tpcloud setup` creates the first one.");
    }
    for u in users {
        println!(
            "  {} {}{}",
            if u.admin { ui::accent("●") } else { ui::dim("●") },
            u.email,
            ui::dim(&format!("{}{}", if u.admin { "  admin" } else { "" }, if u.totp { "  2FA" } else { "" }))
        );
    }
    Ok(())
}
