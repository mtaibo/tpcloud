//! `tpcloud serve`, run by systemd: starts every process the settings call for, restarts any that
//! stops, keeps their output in logs/<name>.log and their state in run/status.json, and stops them
//! all on SIGTERM.

use crate::services::{self, Service, Settings};
use crate::paths;
use anyhow::{Context, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static STOPPING: AtomicBool = AtomicBool::new(false);
const LOG_LIMIT: u64 = 10 * 1024 * 1024;

extern "C" fn on_signal(_: libc::c_int) {
    STOPPING.store(true, Ordering::SeqCst);
}

struct Running {
    service: Service,
    child: Option<Child>,
    started: Instant,
    started_at: u64,
    restarts: u32,
    /// When to start it again after it stopped.
    retry_at: Option<Instant>,
    backoff: Duration,
    last_exit: Option<String>,
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn log_file(name: &str) -> Result<File> {
    let dir = paths::logs_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{name}.log"));
    if std::fs::metadata(&path).map(|m| m.len() > LOG_LIMIT).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join(format!("{name}.log.1")));
    }
    Ok(OpenOptions::new().create(true).append(true).open(path)?)
}

fn spawn(service: &Service) -> Result<Child> {
    std::fs::create_dir_all(&service.cwd)?;
    let mut log = log_file(service.name)?;
    let _ = writeln!(log, "── tpcloud: starting {} ──", service.name);
    let err = log.try_clone()?;
    let mut cmd = Command::new(&service.program);
    cmd.args(&service.args)
        .current_dir(&service.cwd)
        .env_clear()
        .envs(&service.env)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err));
    // Its own process group, so stopping it also stops whatever it started (ffmpeg, rsync).
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }
    cmd.spawn().with_context(|| format!("starting {}", service.name))
}

fn prepare() -> Result<()> {
    let state = paths::state_dir();
    for dir in ["data/auth", "data/store", "home/db/firmwares", "home/mosquitto", "gateway-extra", "caddy", "logs", "run"] {
        std::fs::create_dir_all(state.join(dir))?;
    }
    std::fs::write(state.join("run/mosquitto.conf"), services::mosquitto_conf())?;
    // Mosquitto started as root drops to `nobody`, which must be able to keep its database.
    chown_tree(&state.join("home/mosquitto"), 65534, 65534);
    Ok(())
}

fn chown_tree(path: &std::path::Path, uid: u32, gid: u32) {
    use std::os::unix::ffi::OsStrExt;
    let chown = |p: &std::path::Path| {
        if let Ok(c) = std::ffi::CString::new(p.as_os_str().as_bytes()) {
            unsafe { libc::lchown(c.as_ptr(), uid, gid) };
        }
    };
    chown(path);
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() { chown_tree(&p, uid, gid) } else { chown(&p) }
        }
    }
}

fn write_status(running: &[Running]) {
    let list: Vec<serde_json::Value> = running
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.service.name,
                "running": r.child.is_some(),
                "pid": r.child.as_ref().map(|c| c.id()),
                "since": r.started_at,
                "restarts": r.restarts,
                "last_exit": r.last_exit,
            })
        })
        .collect();
    let body = serde_json::json!({ "pid": std::process::id(), "updated": now_secs(), "services": list });
    let path = paths::status_file();
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, body.to_string()).is_ok() {
        let _ = std::fs::rename(tmp, path);
    }
}

fn stop_child(child: &mut Child) {
    let pid = child.id() as i32;
    unsafe { libc::kill(-pid, libc::SIGTERM) };
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(Some(_)) = child.try_wait() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    unsafe { libc::kill(-pid, libc::SIGKILL) };
    let _ = child.wait();
}

pub fn serve() -> Result<()> {
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as libc::sighandler_t);
        libc::signal(libc::SIGINT, on_signal as libc::sighandler_t);
    }
    prepare()?;
    let settings = Settings::load();
    let mut running: Vec<Running> = services::plan(&settings)
        .into_iter()
        .map(|service| Running {
            service,
            child: None,
            started: Instant::now(),
            started_at: 0,
            restarts: 0,
            retry_at: Some(Instant::now()),
            backoff: Duration::from_secs(1),
            last_exit: None,
        })
        .collect();
    println!("tpcloud {}: supervising {}", env!("CARGO_PKG_VERSION"), running.iter().map(|r| r.service.name).collect::<Vec<_>>().join(", "));

    let mut last_status = Instant::now() - Duration::from_secs(60);
    while !STOPPING.load(Ordering::SeqCst) {
        let mut changed = false;
        for r in running.iter_mut() {
            if let Some(child) = r.child.as_mut() {
                if let Ok(Some(status)) = child.try_wait() {
                    let ran = r.started.elapsed();
                    // A process that ran a while starts again at once; one that keeps failing waits longer each time.
                    r.backoff = if ran > Duration::from_secs(60) { Duration::from_secs(1) } else { (r.backoff * 2).min(Duration::from_secs(30)) };
                    r.last_exit = Some(status.to_string());
                    r.child = None;
                    r.retry_at = Some(Instant::now() + r.backoff);
                    eprintln!("{} stopped ({status}); starting again in {:?}", r.service.name, r.backoff);
                    changed = true;
                }
            }
            if r.child.is_none() && r.retry_at.is_some_and(|t| Instant::now() >= t) {
                match spawn(&r.service) {
                    Ok(child) => {
                        if r.started_at != 0 {
                            r.restarts += 1;
                        }
                        r.child = Some(child);
                        r.started = Instant::now();
                        r.started_at = now_secs();
                        r.retry_at = None;
                    }
                    Err(err) => {
                        eprintln!("{err:#}");
                        r.last_exit = Some(format!("{err:#}"));
                        r.retry_at = Some(Instant::now() + Duration::from_secs(30));
                    }
                }
                changed = true;
            }
        }
        if changed || last_status.elapsed() > Duration::from_secs(5) {
            write_status(&running);
            last_status = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    println!("tpcloud: stopping");
    // The gateway and tunnel first, so nothing new comes in while the rest stops.
    for r in running.iter_mut().rev() {
        if let Some(child) = r.child.as_mut() {
            stop_child(child);
        }
        r.child = None;
    }
    write_status(&running);
    Ok(())
}
