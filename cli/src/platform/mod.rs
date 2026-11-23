//! Everything that depends on the host's operating system, one file per OS with the same
//! functions, so the rest of the program never asks which one it's on. Only Linux exists today;
//! a macOS (launchd, /Library/Application Support) or Windows (a Windows service, %ProgramData%)
//! host adds its own file beside it.
//!
//! What each OS provides:
//! - `APP_DIR`, `STATE_DIR`, `BIN_DIR`: where the runtime, the state and the command live.
//! - `service`: install, enable, start, stop, restart the background service and ask if it runs.
//! - `accounts`: the `tpcloud` group and system user, and adding a person to the group.
//! - `storage`: listing disks, making, opening, mounting and checking a storage pool.
//! - `discovery`: cleaning up what older versions left in avahi's folders.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
