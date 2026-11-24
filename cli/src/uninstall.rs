//! `sudo tpcloud uninstall` on a server. It always asks what goes:
//!
//! - **Only the program** (the default): the runtime, the service and the command go; every
//!   account, file, setting and backup stays, so installing again carries on as it was.
//! - **Everything, with a backup first**: TPCloud's own data (accounts, settings, databases, the
//!   house) is copied to a folder `tpcloud restore` takes, then every trace goes.
//! - **Everything, nothing kept**: as if TPCloud had never been on the machine, the Cloudflare
//!   tunnel it made included.
//!
//! Everyone's files are asked about on their own (they're never in a backup), and nothing is
//! touched until `delete` is typed.

use crate::{backup, client, envfile, install, network, paths, platform, setup, storage, ui};
use anyhow::{bail, Result};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub fn run() -> Result<()> {
    install::require_root("Uninstalling")?;
    if !std::io::stdin().is_terminal() {
        // Nobody to ask: the one that can't lose anything.
        return program_only();
    }
    let env = paths::env_file();
    let name = envfile::get(&env, network::NAME).filter(|n| !n.is_empty()).unwrap_or_else(|| "TPCloud".into());
    println!("\n{}", ui::bold(&format!("Uninstalling {name}")));
    let choice = setup::choose(
        "What should go?",
        &[
            ("Only the program", "accounts, files, settings and backups stay; installing again carries on"),
            ("Everything, with a backup first", "accounts, settings and the house saved to a folder; `tpcloud restore` brings them back"),
            ("Everything, nothing kept", "as if TPCloud had never been here"),
        ],
        0,
    )?;
    if choice == 0 {
        return program_only();
    }
    let keep_backup = choice == 1;
    let state = paths::state_dir();

    let backup_root = if keep_backup {
        let default = backup::dir().unwrap_or_else(|| person_home().unwrap_or_else(|| PathBuf::from("/root")).join("tpcloud-backup"));
        loop {
            let answer = PathBuf::from(setup::ask("Folder for the backup", &default.display().to_string())?);
            if !answer.is_absolute() {
                ui::warn("A full path, starting with /.");
            } else if answer.starts_with(&state) {
                ui::warn(&format!("Not inside {}: that's what's being deleted.", state.display()));
            } else {
                break Some(answer);
            }
        }
    } else {
        None
    };

    // Everyone's files: their own question, since no backup has them.
    let files = envfile::get(&env, "EXTERNAL_DISK_PATH").filter(|f| f.starts_with('/')).map(PathBuf::from).filter(|f| f.is_dir());
    let pool = storage::configured(&env);
    let mut delete_files = false;
    if let Some(dir) = &files {
        let size = backup::du(dir);
        if size == 0 {
            delete_files = true;
        } else {
            delete_files = setup::choose(
                &format!("Everyone's files and photos: {} in {}", backup::human(size), dir.display()),
                &[
                    ("Keep them", "they stay there as plain folders, one per account"),
                    ("Delete them", "for good: they aren't in any backup"),
                ],
                0,
            )? == 1;
        }
        if delete_files && backup_root.as_ref().is_some_and(|b| b.starts_with(dir)) {
            bail!("the backup would be inside the files being deleted; nothing was changed");
        }
    }

    // The nightly backups: kept with a backup, asked about without.
    let nightly = backup::dir().filter(|d| !backup::list(d).is_empty());
    let delete_nightly = match (&nightly, keep_backup) {
        (Some(dir), false) => {
            setup::choose(
                &format!("The nightly backups: {} in {}", backup::list(dir).len(), dir.display()),
                &[("Keep them", "`tpcloud restore` can still bring TPCloud back from one"), ("Delete them", "")],
                0,
            )? == 1
        }
        _ => false,
    };
    let tunnel = envfile::get(&env, network::TUNNEL_ID).filter(|t| !t.is_empty());
    let encrypted = pool.as_ref().is_some_and(|p| !p.luks.is_empty());

    println!();
    println!("  {}", ui::bold("Goes:"));
    let gone = |what: &str| println!("    {} {what}", ui::warn_text("×"));
    gone("the tpcloud and tphome commands, the program and its service");
    gone(&format!("{}: accounts, settings, the photo index, albums, share links, the house", state.display()));
    if let Some(dir) = files.as_ref().filter(|_| delete_files) {
        gone(&format!("{}: everyone's files and photos", dir.display()));
    }
    if let Some(pool) = &pool {
        gone(&format!("mounting the storage pool at {} on every boot", pool.mount.display()));
    }
    if let Some(dir) = nightly.as_ref().filter(|_| delete_nightly) {
        gone(&format!("{}: the nightly backups", dir.display()));
    }
    if tunnel.is_some() && !keep_backup {
        gone("the Cloudflare tunnel this machine made");
    }
    gone("the tpcloud system user and group, and this computer's sign-ins");
    println!("  {}", ui::bold("Stays:"));
    let kept = |what: &str| println!("    {} {what}", ui::green("✓"));
    if let Some(root) = &backup_root {
        kept(&format!("a backup in {}", root.display()));
    }
    if let Some(dir) = files.as_ref().filter(|_| !delete_files) {
        kept(&format!("{}: everyone's files and photos", dir.display()));
        if encrypted && !keep_backup {
            ui::warn("They're on encrypted disks whose key goes with the rest: only the recovery key opens them afterwards.");
        }
    }
    if let Some(dir) = nightly.as_ref().filter(|_| !delete_nightly) {
        kept(&format!("{}: the nightly backups", dir.display()));
    }
    if tunnel.is_some() && keep_backup {
        kept("the Cloudflare tunnel, so the backup works again where it's restored");
    }
    println!();
    if setup::ask("Type delete to go ahead", "")? != "delete" {
        println!("Nothing was changed.");
        return Ok(());
    }

    ui::step("Stopping TPCloud");
    let was_running = platform::service::running();
    if was_running {
        platform::service::stop()?;
    }
    if let Some(root) = &backup_root {
        ui::step(&format!("Backing up to {}", root.display()));
        match backup::make_into(root) {
            Ok(copy) => ui::done(&format!("Saved in {}", copy.display())),
            Err(err) => {
                if was_running {
                    let _ = platform::service::start_once();
                }
                bail!("the backup failed ({err:#}); nothing was removed");
            }
        }
    }
    if let Some(id) = tunnel.as_deref().filter(|_| !keep_backup) {
        ui::step("Deleting the Cloudflare tunnel");
        if let Err(err) = crate::cloudflare::delete(id) {
            ui::warn(&format!("Couldn't ({err:#}): it can be deleted in Cloudflare's dashboard, under Networks → Tunnels."));
        }
    }
    ui::step("Removing the service");
    platform::service::remove()?;
    platform::discovery::withdraw("tpcloud");
    platform::discovery::withdraw("tpcloud-setup");
    platform::discovery::withdraw_host();
    if let Some(dir) = files.as_ref().filter(|_| delete_files) {
        ui::step(&format!("Deleting {}", dir.display()));
        std::fs::remove_dir_all(dir)?;
    }
    if let Some(pool) = &pool {
        ui::step("Taking the storage pool off the boot");
        platform::storage::remove_units(&pool.mount, &pool.luks);
        if delete_files {
            let _ = std::fs::remove_dir(&pool.mount); // only if it's empty, unmounted
        }
    }
    ui::step("Removing the program");
    remove_program()?;
    for path in client::APP_FILES {
        remove_file(Path::new(path))?;
    }
    ui::step(&format!("Removing {}", state.display()));
    remove_state(&state, files.as_deref().filter(|_| !delete_files))?;
    if let Some(dir) = nightly.as_ref().filter(|_| delete_nightly) {
        for copy in backup::list(dir) {
            std::fs::remove_dir_all(copy)?;
        }
        let _ = std::fs::remove_dir(dir);
    }
    client::forget_signins();
    platform::accounts::remove_group_and_user(crate::harden::USER);
    client::remove_empty_dirs(&|_| false);

    ui::done("TPCloud is gone from this machine.");
    if let Some(root) = &backup_root {
        println!("  {}", ui::dim(&format!("To bring it back: install TPCloud, then `sudo tpcloud restore {}/<the copy>`.", root.display())));
    }
    if let (Some(pool), false) = (&pool, delete_files) {
        println!("  {}", ui::dim(&format!("The pool's disks keep the files; they're no longer mounted at {} by themselves.", pool.mount.display())));
        if !pool.luks.is_empty() {
            println!("  {}", ui::dim("To read them: `cryptsetup open` each disk with its recovery key, then mount one of them."));
        }
    }
    if pool.is_some() && delete_files {
        println!("  {}", ui::dim("The pool's disks are empty now; `wipefs -a` each one to use it for something else."));
    }
    if tunnel.is_some() && !keep_backup {
        println!("  {}", ui::dim("Its DNS record stays in Cloudflare, pointing at nothing: delete it there if you want."));
    }
    Ok(())
}

/// The command (and `tphome`) and the runtime.
fn remove_program() -> Result<()> {
    for name in ["tpcloud", "tphome"] {
        remove_file(&paths::bin_dir().join(name))?;
    }
    let app = paths::app_dir();
    if app.exists() {
        std::fs::remove_dir_all(&app)?;
    }
    Ok(())
}

fn remove_file(path: &Path) -> Result<()> {
    if path.symlink_metadata().is_ok() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// The state folder, all of it, or all but the files kept inside it (and the folders leading to them).
fn remove_state(state: &Path, keep: Option<&Path>) -> Result<()> {
    if !state.exists() {
        return Ok(());
    }
    match keep.filter(|k| k.starts_with(state) && *k != state) {
        None => std::fs::remove_dir_all(state)?,
        Some(keep) => remove_all_but(state, keep)?,
    }
    Ok(())
}

fn remove_all_but(dir: &Path, keep: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path == keep {
            continue;
        }
        if keep.starts_with(&path) {
            remove_all_but(&path, keep)?;
        } else if path.symlink_metadata()?.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Under sudo, the home of whoever ran it.
fn person_home() -> Option<PathBuf> {
    client::person_home()
}

/// Only the program: the one that can't lose anything.
fn program_only() -> Result<()> {
    let state = paths::state_dir();
    let files = envfile::get(&paths::env_file(), "EXTERNAL_DISK_PATH").unwrap_or_default();
    ui::step("Stopping TPCloud");
    platform::service::remove()?;
    platform::discovery::withdraw("tpcloud");
    platform::discovery::withdraw("tpcloud-setup");
    platform::discovery::withdraw_host();
    ui::step("Removing the program");
    remove_program()?;
    ui::done("TPCloud is uninstalled.");
    println!("  {}", ui::dim("Kept, untouched:"));
    println!("  {}", ui::dim(&format!("  {}  accounts, settings, the photo index, albums, share links, the house", state.display())));
    if !files.is_empty() {
        println!("  {}", ui::dim(&format!("  {files}  everyone's files and photos")));
    }
    if let Some(dir) = backup::dir() {
        println!("  {}", ui::dim(&format!("  {}  backups", dir.display())));
    }
    if storage::configured(&paths::env_file()).is_some() {
        println!("  {}", ui::dim("  the storage pool, still mounted at every boot"));
    }
    println!("  {}", ui::dim("Installing TPCloud again finds all of it and carries on."));
    println!("  {}", ui::dim(&format!("To remove it too: `curl -fsSL https://tpcloud.migueltaibo.com/get | sh -s -- --uninstall` (or `sudo rm -rf {}`).", state.display())));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_the_files_inside_the_state() {
        let root = std::env::temp_dir().join(format!("tpcloud-uninstall-{}", std::process::id()));
        let state = root.join("state");
        for dir in ["data/auth", "files/a@b.c/photos", "logs"] {
            std::fs::create_dir_all(state.join(dir)).unwrap();
        }
        std::fs::write(state.join("tpcloud.env"), "x").unwrap();
        std::fs::write(state.join("files/a@b.c/photos/1.jpg"), "x").unwrap();
        remove_state(&state, Some(&state.join("files"))).unwrap();
        let left: Vec<_> = std::fs::read_dir(&state).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(left, vec![std::ffi::OsString::from("files")]);
        assert!(state.join("files/a@b.c/photos/1.jpg").exists());
        remove_state(&state, None).unwrap();
        assert!(!state.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
