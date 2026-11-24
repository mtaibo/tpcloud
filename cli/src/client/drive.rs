//! Folders on demand (Linux): the folder shows every file in the cloud, with its name, size and
//! date, but they take no room here until one is opened; then it's fetched from the server and
//! kept for a day (or for good, inside folders marked to be kept here). Files written here go up
//! when they're closed. It's a FUSE filesystem mounted at the folder by the sync service.
//!
//! What's here: `~/.cache/tpcloud/<folder id>/files/` holds the fetched files under their own
//! paths, `index.json` when each was last opened, and `tree.json` the last listing from the
//! server, so the folder can still be browsed (and its fetched files opened) without a network.
//! Names starting with a dot (editors' swap files, lock files) stay on this computer only, like
//! the server hides them anyway.

use super::net::{query_value, Remote};
use super::sync::Pair;
use fuser::{
    Errno, FileAttr, FileHandle, FileType, Filesystem, FopenFlags, Generation, INodeNo, LockOwner, MountOption, OpenAccMode, OpenFlags, RenameFlags, ReplyAttr, ReplyCreate, ReplyData,
    ReplyDirectory, ReplyEmpty, ReplyEntry, ReplyOpen, ReplyStatfs, ReplyWrite, Request, TimeOrNow, WriteFlags,
};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const API: &str = "/store/api/files";
const TTL: Duration = Duration::from_secs(1);
/// A fetched file stays this long after it was last opened (unless it's kept here for good).
pub const KEEP_FOR: Duration = Duration::from_secs(24 * 3600);
const REFRESH_EVERY: Duration = Duration::from_secs(30);

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn time_of(secs: i64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs.max(0) as u64)
}

fn errno(e: i32) -> Errno {
    Errno::from_i32(e)
}

/// Kept on this computer only: any part of the path starts with a dot.
pub fn local_only(path: &str) -> bool {
    path.split('/').any(|p| p.starts_with('.'))
}

fn parent_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(p, _)| p)
}

fn name_of(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, n)| n)
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Under `prefix` (or it): "a/b" is under "a", "ab" isn't.
pub fn under(path: &str, prefix: &str) -> bool {
    prefix.is_empty() || path == prefix || path.starts_with(&format!("{prefix}/"))
}

// ── Where things are kept ──

pub fn cache_root(id: &str) -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp")).join(".cache"));
    base.join("tpcloud").join(id)
}

pub fn files_dir(id: &str) -> PathBuf {
    cache_root(id).join("files")
}

/// Bytes kept here for a folder (fetched or kept for good).
pub fn cached_bytes(id: &str) -> u64 {
    fn walk(dir: &Path) -> u64 {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| match e.metadata() {
                Ok(m) if m.is_dir() => walk(&e.path()),
                Ok(m) => m.len(),
                Err(_) => 0,
            })
            .sum()
    }
    walk(&files_dir(id))
}

/// Takes away what was fetched under `rel` ("" = all of it), unless it's in `keep`: the
/// `tpcloud sync free` and the terminal app's «free up space».
pub fn free(id: &str, rel: &str, keep: &[String]) -> u64 {
    let root = files_dir(id);
    let mut freed = 0;
    let mut stack = vec![root.join(rel)];
    while let Some(p) = stack.pop() {
        let Ok(meta) = p.symlink_metadata() else { continue };
        let relpath = p.strip_prefix(&root).map(|r| r.to_string_lossy().into_owned()).unwrap_or_default();
        if keep.iter().any(|k| under(&relpath, k)) {
            continue;
        }
        if meta.is_dir() {
            stack.extend(std::fs::read_dir(&p).into_iter().flatten().flatten().map(|e| e.path()));
        } else if !local_only(&relpath) && std::fs::remove_file(&p).is_ok() {
            freed += meta.len();
        }
    }
    freed
}

// ── The tree: what the folder shows ──

#[derive(Clone, Debug, PartialEq)]
struct Node {
    dir: bool,
    size: u64,
    mtime: i64,
}

#[derive(Default)]
struct Tree {
    nodes: HashMap<String, Node>,
    children: HashMap<String, BTreeSet<String>>,
}

impl Tree {
    fn new() -> Tree {
        let mut t = Tree::default();
        t.nodes.insert(String::new(), Node { dir: true, size: 0, mtime: now_secs() });
        t
    }

    fn insert(&mut self, path: &str, node: Node) {
        if path.is_empty() {
            return;
        }
        let parent = parent_of(path).to_string();
        if !self.nodes.contains_key(&parent) {
            self.insert(&parent, Node { dir: true, size: 0, mtime: node.mtime });
        }
        self.children.entry(parent).or_default().insert(name_of(path).to_string());
        self.nodes.insert(path.to_string(), node);
    }

    /// The node and everything under it.
    fn remove(&mut self, path: &str) {
        if let Some(kids) = self.children.remove(path) {
            for k in kids {
                self.remove(&join(path, &k));
            }
        }
        self.nodes.remove(path);
        if let Some(siblings) = self.children.get_mut(parent_of(path)) {
            siblings.remove(name_of(path));
        }
    }

    fn rename(&mut self, from: &str, to: &str) {
        let moved: Vec<(String, Node)> = self.nodes.iter().filter(|(p, _)| under(p, from)).map(|(p, n)| (p.clone(), n.clone())).collect();
        self.remove(from);
        for (p, n) in moved {
            let new = format!("{to}{}", &p[from.len()..]);
            self.insert(&new, n);
        }
    }

    fn from_listing(files: &[(String, u64, i64)], dirs: &[String]) -> Tree {
        let mut t = Tree::new();
        for d in dirs {
            t.insert(d, Node { dir: true, size: 0, mtime: now_secs() });
        }
        for (p, size, mtime) in files {
            t.insert(p, Node { dir: false, size: *size, mtime: *mtime });
        }
        t
    }

    fn to_json(&self) -> Value {
        let files: Vec<Value> = self.nodes.iter().filter(|(p, n)| !n.dir && !local_only(p)).map(|(p, n)| json!([p, n.size, n.mtime])).collect();
        let dirs: Vec<&String> = self.nodes.iter().filter(|(p, n)| n.dir && !p.is_empty() && !local_only(p)).map(|(p, _)| p).collect();
        json!({ "files": files, "dirs": dirs })
    }

    fn from_json(v: &Value) -> Tree {
        let files: Vec<(String, u64, i64)> = v["files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| Some((f[0].as_str()?.to_string(), f[1].as_u64()?, f[2].as_i64()?)))
            .collect();
        let dirs: Vec<String> = v["dirs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(str::to_string)).collect();
        Tree::from_listing(&files, &dirs)
    }
}

struct Inodes {
    by_path: HashMap<String, u64>,
    by_ino: HashMap<u64, String>,
    next: u64,
}

impl Inodes {
    fn new() -> Inodes {
        let mut i = Inodes { by_path: HashMap::new(), by_ino: HashMap::new(), next: 2 };
        i.by_path.insert(String::new(), 1);
        i.by_ino.insert(1, String::new());
        i
    }

    fn of(&mut self, path: &str) -> u64 {
        if let Some(i) = self.by_path.get(path) {
            return *i;
        }
        let i = self.next;
        self.next += 1;
        self.by_path.insert(path.to_string(), i);
        self.by_ino.insert(i, path.to_string());
        i
    }

    fn path(&self, ino: u64) -> Option<String> {
        self.by_ino.get(&ino).cloned()
    }

    fn rename(&mut self, from: &str, to: &str) {
        let moved: Vec<(String, u64)> = self.by_path.iter().filter(|(p, _)| under(p, from)).map(|(p, i)| (p.clone(), *i)).collect();
        for (p, i) in moved {
            let new = format!("{to}{}", &p[from.len()..]);
            self.by_path.remove(&p);
            self.by_path.insert(new.clone(), i);
            self.by_ino.insert(i, new);
        }
    }
}

struct Handle {
    path: String,
    file: File,
    dirty: bool,
}

/// One folder on demand.
pub struct Drive {
    pair: Pair,
    remote: Mutex<Option<Arc<Remote>>>,
    tree: RwLock<Tree>,
    inodes: Mutex<Inodes>,
    handles: Mutex<HashMap<u64, Handle>>,
    next_fh: AtomicU64,
    /// When each fetched file was last opened.
    opened: Mutex<HashMap<String, i64>>,
    pub pinned: RwLock<Vec<String>>,
    /// Something changed (what's kept here): housekeeping now rather than in half a minute.
    pub poke: std::sync::atomic::AtomicBool,
    uid: u32,
    gid: u32,
}

impl Drive {
    pub fn new(pair: Pair) -> Arc<Drive> {
        let root = cache_root(&pair.id);
        let _ = std::fs::create_dir_all(root.join("files"));
        let tree = std::fs::read_to_string(root.join("tree.json")).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| Tree::from_json(&v)).unwrap_or_else(Tree::new);
        let opened: HashMap<String, i64> = std::fs::read_to_string(root.join("index.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v.as_object().map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_i64()?))).collect()))
            .unwrap_or_default();
        let drive = Arc::new(Drive {
            pinned: RwLock::new(pair.pinned.clone()),
            pair,
            remote: Mutex::new(None),
            tree: RwLock::new(tree),
            inodes: Mutex::new(Inodes::new()),
            handles: Mutex::new(HashMap::new()),
            next_fh: AtomicU64::new(1),
            opened: Mutex::new(opened),
            poke: std::sync::atomic::AtomicBool::new(false),
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
        });
        drive.adopt_local_only();
        drive
    }

    /// Dot files left in the cache from before (an editor's swap file): back in the tree.
    fn adopt_local_only(&self) {
        let root = files_dir(&self.pair.id);
        let mut stack = vec![root.clone()];
        let mut tree = self.tree.write().unwrap();
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(meta) = e.metadata() else { continue };
                let rel = e.path().strip_prefix(&root).map(|r| r.to_string_lossy().into_owned()).unwrap_or_default();
                if meta.is_dir() {
                    stack.push(e.path());
                }
                if local_only(&rel) {
                    tree.insert(&rel, Node { dir: meta.is_dir(), size: meta.len(), mtime: mtime_of(&meta) });
                }
            }
        }
    }

    fn cached(&self, path: &str) -> PathBuf {
        files_dir(&self.pair.id).join(path)
    }

    fn remote_path(&self, rel: &str) -> String {
        if rel.is_empty() {
            self.pair.remote.clone()
        } else {
            format!("{}/{rel}", self.pair.remote)
        }
    }

    /// The server, connecting again if the last try failed.
    fn remote(&self) -> Result<Arc<Remote>, Errno> {
        if let Some(r) = self.remote.lock().unwrap().clone() {
            return Ok(r);
        }
        match super::connect_network(&self.pair.network) {
            Ok((r, _)) => {
                let r = Arc::new(r);
                *self.remote.lock().unwrap() = Some(r.clone());
                Ok(r)
            }
            Err(_) => Err(errno(libc::EHOSTUNREACH)),
        }
    }

    /// A request failed: next time, connect again (the address may have changed).
    fn offline(&self) {
        *self.remote.lock().unwrap() = None;
    }

    // ── Keeping up with the server ──

    /// The server's listing now; what's open or only here is kept as it is.
    pub fn refresh(&self) -> anyhow::Result<()> {
        let remote = self.remote().map_err(|_| anyhow::anyhow!("offline"))?;
        let tree = match remote.get(&format!("{API}/tree?path={}", query_value(&self.pair.remote))) {
            Ok(t) => t,
            Err(e) => {
                self.offline();
                return Err(e);
            }
        };
        let files: Vec<(String, u64, i64)> = tree["files"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| Some((f["path"].as_str()?.to_string(), f["size"].as_u64()?, f["modified"].as_f64()?.floor() as i64)))
            .collect();
        let dirs: Vec<String> = tree["dirs"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(str::to_string)).collect();
        let mut fresh = Tree::from_listing(&files, &dirs);
        let open: Vec<String> = self.handles.lock().unwrap().values().map(|h| h.path.clone()).collect();
        {
            let old = self.tree.read().unwrap();
            for (p, n) in old.nodes.iter() {
                if local_only(p) || open.contains(p) {
                    fresh.insert(p, n.clone());
                }
            }
        }
        // A fetched file that changed on the server: fetched again next time it's opened.
        for (p, size, mtime) in &files {
            let c = self.cached(p);
            if let Ok(m) = c.metadata() {
                if (m.len() != *size || mtime_of(&m) != *mtime) && !open.contains(p) {
                    let _ = std::fs::remove_file(&c);
                }
            }
        }
        let json = fresh.to_json();
        *self.tree.write().unwrap() = fresh;
        let _ = std::fs::write(cache_root(&self.pair.id).join("tree.json"), json.to_string());
        Ok(())
    }

    /// Fetches what's in folders kept here and isn't yet; forgets what wasn't opened in a day.
    pub fn housekeeping(&self) {
        let pinned = self.pinned.read().unwrap().clone();
        let wanted: Vec<String> = {
            let tree = self.tree.read().unwrap();
            tree.nodes.iter().filter(|(p, n)| !n.dir && !local_only(p) && pinned.iter().any(|k| under(p, k))).map(|(p, _)| p.clone()).collect()
        };
        for p in wanted {
            if self.ensure_cached(&p).is_err() {
                break; // offline: next time
            }
        }
        let open: Vec<String> = self.handles.lock().unwrap().values().map(|h| h.path.clone()).collect();
        let limit = now_secs() - KEEP_FOR.as_secs() as i64;
        let root = files_dir(&self.pair.id);
        let mut opened = self.opened.lock().unwrap();
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let Ok(meta) = e.metadata() else { continue };
                let rel = e.path().strip_prefix(&root).map(|r| r.to_string_lossy().into_owned()).unwrap_or_default();
                if meta.is_dir() {
                    stack.push(e.path());
                    continue;
                }
                if local_only(&rel) || open.contains(&rel) || pinned.iter().any(|k| under(&rel, k)) || rel.ends_with(".tpcloud-part") {
                    continue;
                }
                // Not known (moved here when the folder became on demand): counts from now.
                let last = *opened.entry(rel.clone()).or_insert_with(now_secs);
                if last < limit && std::fs::remove_file(e.path()).is_ok() {
                    opened.remove(&rel);
                }
            }
        }
        let index: serde_json::Map<String, Value> = opened.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let _ = std::fs::write(cache_root(&self.pair.id).join("index.json"), Value::Object(index).to_string());
    }

    /// The file here, fetched first if it isn't (or changed since).
    fn ensure_cached(&self, path: &str) -> Result<PathBuf, Errno> {
        let node = self.tree.read().unwrap().nodes.get(path).cloned().ok_or(Errno::ENOENT)?;
        let local = self.cached(path);
        let fresh = local.metadata().is_ok_and(|m| local_only(path) || (m.len() == node.size && mtime_of(&m) == node.mtime));
        if !fresh {
            if local_only(path) {
                return Err(Errno::ENOENT);
            }
            if let Some(dir) = local.parent() {
                std::fs::create_dir_all(dir).map_err(|_| Errno::EIO)?;
            }
            let remote = self.remote()?;
            if remote.download_to(&format!("{API}/download?path={}", query_value(&self.remote_path(path))), &local).is_err() {
                self.offline();
                return Err(Errno::EIO);
            }
            let _ = set_mtime(&local, node.mtime);
        }
        self.opened.lock().unwrap().insert(path.to_string(), now_secs());
        Ok(local)
    }

    /// A file written here, sent up with its time; on failure it waits for the next try.
    fn upload(&self, path: &str) -> Result<(), Errno> {
        if local_only(path) {
            return Ok(());
        }
        let local = self.cached(path);
        let meta = local.metadata().map_err(|_| Errno::ENOENT)?;
        let mtime = mtime_of(&meta);
        let remote = self.remote()?;
        let dir = self.remote_path(parent_of(path));
        if remote.upload_file(&format!("{API}/upload?path={}&mtime={mtime}", query_value(&dir)), "files", &local).is_err() {
            self.offline();
            self.mark_pending(path, true);
            return Err(Errno::EIO);
        }
        self.mark_pending(path, false);
        self.tree.write().unwrap().insert(path, Node { dir: false, size: meta.len(), mtime });
        self.opened.lock().unwrap().insert(path.to_string(), now_secs());
        Ok(())
    }

    fn mark_pending(&self, path: &str, pending: bool) {
        let file = cache_root(&self.pair.id).join("pending.json");
        let mut list: Vec<String> = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        list.retain(|p| p != path);
        if pending {
            list.push(path.to_string());
        }
        let _ = std::fs::write(&file, serde_json::to_string(&list).unwrap_or_default());
    }

    /// Files written while offline, sent now.
    pub fn retry_pending(&self) {
        let file = cache_root(&self.pair.id).join("pending.json");
        let list: Vec<String> = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        for p in list {
            if self.upload(&p).is_err() {
                return;
            }
        }
    }

    fn send(&self, method: &str, path: &str, body: Option<Value>) -> Result<(), Errno> {
        let remote = self.remote()?;
        match remote.send(method, path, body) {
            Ok(_) => Ok(()),
            Err(e) if e.to_string().starts_with("couldn't reach") => {
                self.offline();
                Err(errno(libc::EHOSTUNREACH))
            }
            Err(_) => Err(Errno::EIO),
        }
    }

    fn attr(&self, ino: u64, node: &Node) -> FileAttr {
        let t = time_of(node.mtime);
        FileAttr {
            ino: INodeNo(ino),
            size: node.size,
            // What it takes here: nothing until fetched (du and file managers show the truth).
            blocks: if node.dir { 0 } else { self.cached(&self.inodes.lock().unwrap().path(ino).unwrap_or_default()).metadata().map(|m| (m.len() + 511) / 512).unwrap_or(0) },
            atime: t,
            mtime: t,
            ctime: t,
            crtime: t,
            kind: if node.dir { FileType::Directory } else { FileType::RegularFile },
            perm: if node.dir { 0o755 } else { 0o644 },
            nlink: if node.dir { 2 } else { 1 },
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            flags: 0,
            blksize: 4096,
        }
    }

    fn node_of(&self, ino: INodeNo) -> Option<(String, Node)> {
        let path = self.inodes.lock().unwrap().path(u64::from(ino))?;
        let node = self.tree.read().unwrap().nodes.get(&path).cloned()?;
        Some((path, node))
    }

    fn child(&self, parent: INodeNo, name: &OsStr) -> Option<String> {
        let dir = self.inodes.lock().unwrap().path(u64::from(parent))?;
        Some(join(&dir, name.to_str()?))
    }

    fn entry(&self, path: &str) -> Option<(u64, FileAttr)> {
        let node = self.tree.read().unwrap().nodes.get(path).cloned()?;
        let ino = self.inodes.lock().unwrap().of(path);
        Some((ino, self.attr(ino, &node)))
    }

    fn new_handle(&self, path: &str, file: File, dirty: bool) -> u64 {
        let fh = self.next_fh.fetch_add(1, Ordering::Relaxed);
        self.handles.lock().unwrap().insert(fh, Handle { path: path.to_string(), file, dirty });
        fh
    }

    fn rename_cached(&self, from: &str, to: &str) {
        let (a, b) = (self.cached(from), self.cached(to));
        if a.exists() {
            if let Some(dir) = b.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::rename(&a, &b);
        }
        let mut opened = self.opened.lock().unwrap();
        let moved: Vec<(String, i64)> = opened.iter().filter(|(p, _)| under(p, from)).map(|(p, t)| (p.clone(), *t)).collect();
        for (p, t) in moved {
            opened.remove(&p);
            opened.insert(format!("{to}{}", &p[from.len()..]), t);
        }
        for h in self.handles.lock().unwrap().values_mut() {
            if under(&h.path, from) {
                h.path = format!("{to}{}", &h.path[from.len()..]);
            }
        }
    }
}

fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn set_mtime(path: &Path, mtime: i64) -> std::io::Result<()> {
    File::options().write(true).open(path)?.set_modified(time_of(mtime))
}

/// The filesystem itself, a thin layer over the Drive (shared with the service's housekeeping).
pub struct DriveFs(pub Arc<Drive>);

impl Filesystem for DriveFs {
    fn lookup(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEntry) {
        match self.0.child(parent, name).and_then(|p| self.0.entry(&p)) {
            Some((_, attr)) => reply.entry(&TTL, &attr, Generation(0)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn getattr(&self, _req: &Request, ino: INodeNo, fh: Option<FileHandle>, reply: ReplyAttr) {
        let Some((path, mut node)) = self.0.node_of(ino) else { return reply.error(Errno::ENOENT) };
        // While it's being written, its size is the file's here.
        if let Some(h) = fh.and_then(|fh| self.0.handles.lock().unwrap().get(&u64::from(fh)).and_then(|h| h.file.metadata().ok())) {
            node.size = h.len();
        } else if self.0.handles.lock().unwrap().values().any(|h| h.path == path && h.dirty) {
            if let Ok(m) = self.0.cached(&path).metadata() {
                node.size = m.len();
            }
        }
        reply.attr(&TTL, &self.0.attr(u64::from(ino), &node));
    }

    fn setattr(
        &self,
        _req: &Request,
        ino: INodeNo,
        _mode: Option<u32>,
        _uid: Option<u32>,
        _gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<TimeOrNow>,
        mtime: Option<TimeOrNow>,
        _ctime: Option<SystemTime>,
        fh: Option<FileHandle>,
        _crtime: Option<SystemTime>,
        _chgtime: Option<SystemTime>,
        _bkuptime: Option<SystemTime>,
        _flags: Option<fuser::BsdFileFlags>,
        _kill_suid_gid: bool,
        reply: ReplyAttr,
    ) {
        let Some((path, mut node)) = self.0.node_of(ino) else { return reply.error(Errno::ENOENT) };
        if node.dir {
            return reply.attr(&TTL, &self.0.attr(u64::from(ino), &node));
        }
        if let Some(size) = size {
            let open = fh.map(u64::from).filter(|fh| self.0.handles.lock().unwrap().contains_key(fh));
            match open {
                Some(fh) => {
                    let mut handles = self.0.handles.lock().unwrap();
                    let h = handles.get_mut(&fh).unwrap();
                    if h.file.set_len(size).is_err() {
                        return reply.error(Errno::EIO);
                    }
                    h.dirty = true;
                }
                None => {
                    // Truncating a file that isn't open: here, then up at once.
                    let local = if size == 0 {
                        let l = self.0.cached(&path);
                        if let Some(d) = l.parent() {
                            let _ = std::fs::create_dir_all(d);
                        }
                        l
                    } else {
                        match self.0.ensure_cached(&path) {
                            Ok(l) => l,
                            Err(e) => return reply.error(e),
                        }
                    };
                    if File::options().write(true).create(true).truncate(false).open(&local).and_then(|f| f.set_len(size)).is_err() {
                        return reply.error(Errno::EIO);
                    }
                    if let Err(e) = self.0.upload(&path) {
                        return reply.error(e);
                    }
                }
            }
            node.size = size;
            node.mtime = now_secs();
            self.0.tree.write().unwrap().insert(&path, node.clone());
        }
        if let Some(t) = mtime {
            let secs = match t {
                TimeOrNow::Now => now_secs(),
                TimeOrNow::SpecificTime(t) => t.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0),
            };
            let _ = set_mtime(&self.0.cached(&path), secs);
            node.mtime = secs;
            self.0.tree.write().unwrap().insert(&path, node.clone());
        }
        reply.attr(&TTL, &self.0.attr(u64::from(ino), &node));
    }

    fn mkdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, _mode: u32, _umask: u32, _owner: fuser::Owner, reply: ReplyEntry) {
        let Some(path) = self.0.child(parent, name) else { return reply.error(Errno::ENOENT) };
        if self.0.tree.read().unwrap().nodes.contains_key(&path) {
            return reply.error(errno(libc::EEXIST));
        }
        if local_only(&path) {
            let _ = std::fs::create_dir_all(self.0.cached(&path));
        } else if let Err(e) = self.0.send("POST", &format!("{API}/mkdir"), Some(json!({ "path": self.0.remote_path(&path) }))) {
            return reply.error(e);
        }
        self.0.tree.write().unwrap().insert(&path, Node { dir: true, size: 0, mtime: now_secs() });
        match self.0.entry(&path) {
            Some((_, attr)) => reply.entry(&TTL, &attr, Generation(0)),
            None => reply.error(Errno::EIO),
        }
    }

    fn unlink(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(path) = self.0.child(parent, name) else { return reply.error(Errno::ENOENT) };
        if !self.0.tree.read().unwrap().nodes.contains_key(&path) {
            return reply.error(Errno::ENOENT);
        }
        if !local_only(&path) {
            if let Err(e) = self.0.send("DELETE", &format!("{API}?path={}", query_value(&self.0.remote_path(&path))), None) {
                return reply.error(e);
            }
        }
        let _ = std::fs::remove_file(self.0.cached(&path));
        self.0.opened.lock().unwrap().remove(&path);
        self.0.tree.write().unwrap().remove(&path);
        reply.ok();
    }

    fn rmdir(&self, _req: &Request, parent: INodeNo, name: &OsStr, reply: ReplyEmpty) {
        let Some(path) = self.0.child(parent, name) else { return reply.error(Errno::ENOENT) };
        if self.0.tree.read().unwrap().children.get(&path).is_some_and(|c| !c.is_empty()) {
            return reply.error(Errno::ENOTEMPTY);
        }
        if !local_only(&path) {
            if let Err(e) = self.0.send("DELETE", &format!("{API}?path={}", query_value(&self.0.remote_path(&path))), None) {
                return reply.error(e);
            }
        }
        let _ = std::fs::remove_dir_all(self.0.cached(&path));
        self.0.tree.write().unwrap().remove(&path);
        reply.ok();
    }

    fn rename(&self, _req: &Request, parent: INodeNo, name: &OsStr, newparent: INodeNo, newname: &OsStr, _flags: RenameFlags, _owner: Option<fuser::Owner>, reply: ReplyEmpty) {
        let (Some(from), Some(to)) = (self.0.child(parent, name), self.0.child(newparent, newname)) else { return reply.error(Errno::ENOENT) };
        let Some(node) = self.0.tree.read().unwrap().nodes.get(&from).cloned() else { return reply.error(Errno::ENOENT) };
        let target = self.0.tree.read().unwrap().nodes.get(&to).cloned();
        if let Some(t) = &target {
            if t.dir && self.0.tree.read().unwrap().children.get(&to).is_some_and(|c| !c.is_empty()) {
                return reply.error(Errno::ENOTEMPTY);
            }
        }
        let d = &self.0;
        let outcome: Result<(), Errno> = (|| {
            match (local_only(&from), local_only(&to)) {
                (true, true) => {}
                // An editor saving: it wrote «.name.swp» (here only) and renames it over the real one.
                (true, false) => {
                    d.rename_cached(&from, &to);
                    if !node.dir {
                        d.tree.write().unwrap().insert(&to, node.clone());
                        d.upload(&to)?;
                    }
                }
                // Hidden from now on: kept here, gone from the server.
                (false, true) => {
                    if !node.dir {
                        d.ensure_cached(&from)?;
                    }
                    d.send("DELETE", &format!("{API}?path={}", query_value(&d.remote_path(&from))), None)?;
                    d.rename_cached(&from, &to);
                }
                (false, false) => {
                    if target.is_some() {
                        d.send("DELETE", &format!("{API}?path={}", query_value(&d.remote_path(&to))), None)?;
                    }
                    let (fdir, tdir) = (parent_of(&from), parent_of(&to));
                    let mut at = from.clone();
                    if fdir != tdir {
                        d.send("POST", &format!("{API}/move"), Some(json!({ "path": d.remote_path(&from), "dest_dir": d.remote_path(tdir) })))?;
                        at = join(tdir, name_of(&from));
                    }
                    if name_of(&from) != name_of(&to) {
                        d.send("POST", &format!("{API}/rename"), Some(json!({ "path": d.remote_path(&at), "new_name": name_of(&to) })))?;
                    }
                    d.rename_cached(&from, &to);
                }
            }
            Ok(())
        })();
        if let Err(e) = outcome {
            return reply.error(e);
        }
        let mut tree = self.0.tree.write().unwrap();
        tree.remove(&to);
        tree.rename(&from, &to);
        drop(tree);
        self.0.inodes.lock().unwrap().rename(&from, &to);
        reply.ok();
    }

    fn open(&self, _req: &Request, ino: INodeNo, flags: OpenFlags, _kill_suid_gid: bool, reply: ReplyOpen) {
        let Some((path, node)) = self.0.node_of(ino) else { return reply.error(Errno::ENOENT) };
        if node.dir {
            return reply.error(errno(libc::EISDIR));
        }
        let writing = flags.acc_mode() != OpenAccMode::O_RDONLY;
        let truncate = flags.0 & libc::O_TRUNC != 0;
        let local = if truncate {
            let l = self.0.cached(&path);
            if let Some(d) = l.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            l
        } else {
            match self.0.ensure_cached(&path) {
                Ok(l) => l,
                Err(e) => return reply.error(e),
            }
        };
        let file = File::options().read(true).write(writing).create(truncate).truncate(truncate).open(&local);
        match file {
            Ok(f) => {
                let fh = self.0.new_handle(&path, f, truncate);
                reply.opened(FileHandle(fh), FopenFlags::empty());
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn read(&self, _req: &Request, _ino: INodeNo, fh: FileHandle, offset: u64, size: u32, _flags: OpenFlags, _lock: Option<LockOwner>, reply: ReplyData) {
        let handles = self.0.handles.lock().unwrap();
        let Some(h) = handles.get(&u64::from(fh)) else { return reply.error(errno(libc::EBADF)) };
        let mut buf = vec![0u8; size as usize];
        match h.file.read_at(&mut buf, offset) {
            Ok(n) => reply.data(&buf[..n]),
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn write(&self, _req: &Request, _ino: INodeNo, fh: FileHandle, offset: u64, data: &[u8], _wf: WriteFlags, _flags: OpenFlags, _lock: Option<LockOwner>, reply: ReplyWrite) {
        let mut handles = self.0.handles.lock().unwrap();
        let Some(h) = handles.get_mut(&u64::from(fh)) else { return reply.error(errno(libc::EBADF)) };
        match h.file.write_all_at(data, offset) {
            Ok(()) => {
                h.dirty = true;
                reply.written(data.len() as u32);
            }
            Err(_) => reply.error(Errno::EIO),
        }
    }

    fn flush(&self, _req: &Request, _ino: INodeNo, _fh: FileHandle, _lock: LockOwner, reply: ReplyEmpty) {
        reply.ok();
    }

    fn fsync(&self, _req: &Request, _ino: INodeNo, fh: FileHandle, _datasync: bool, reply: ReplyEmpty) {
        if let Some(h) = self.0.handles.lock().unwrap().get(&u64::from(fh)) {
            let _ = h.file.sync_all();
        }
        reply.ok();
    }

    fn release(&self, _req: &Request, _ino: INodeNo, fh: FileHandle, _flags: OpenFlags, _lock: Option<LockOwner>, _flush: bool, reply: ReplyEmpty) {
        let handle = self.0.handles.lock().unwrap().remove(&u64::from(fh));
        reply.ok();
        // Closed: if it was written, up it goes (the program already got its answer).
        if let Some(h) = handle {
            if h.dirty {
                let _ = h.file.sync_all();
                drop(h.file);
                if let Ok(m) = self.0.cached(&h.path).metadata() {
                    self.0.tree.write().unwrap().insert(&h.path, Node { dir: false, size: m.len(), mtime: mtime_of(&m) });
                }
                let _ = self.0.upload(&h.path);
            }
        }
    }

    fn opendir(&self, _req: &Request, ino: INodeNo, _flags: OpenFlags, reply: ReplyOpen) {
        match self.0.node_of(ino) {
            Some((_, n)) if n.dir => reply.opened(FileHandle(0), FopenFlags::empty()),
            Some(_) => reply.error(errno(libc::ENOTDIR)),
            None => reply.error(Errno::ENOENT),
        }
    }

    fn readdir(&self, _req: &Request, ino: INodeNo, _fh: FileHandle, offset: u64, mut reply: ReplyDirectory) {
        let Some((path, _)) = self.0.node_of(ino) else { return reply.error(Errno::ENOENT) };
        let kids: Vec<String> = self.0.tree.read().unwrap().children.get(&path).map(|c| c.iter().cloned().collect()).unwrap_or_default();
        let mut entries: Vec<(u64, FileType, String)> = vec![(u64::from(ino), FileType::Directory, ".".into()), (1, FileType::Directory, "..".into())];
        for k in kids {
            let p = join(&path, &k);
            let dir = self.0.tree.read().unwrap().nodes.get(&p).is_some_and(|n| n.dir);
            let i = self.0.inodes.lock().unwrap().of(&p);
            entries.push((i, if dir { FileType::Directory } else { FileType::RegularFile }, k));
        }
        for (i, (ino, kind, name)) in entries.into_iter().enumerate().skip(offset as usize) {
            if reply.add(INodeNo(ino), (i + 1) as u64, kind, name) {
                break;
            }
        }
        reply.ok();
    }

    fn statfs(&self, _req: &Request, _ino: INodeNo, reply: ReplyStatfs) {
        // The cloud's room isn't known here: plenty, so programs don't refuse to save.
        reply.statfs(1 << 32, 1 << 31, 1 << 31, 1 << 20, 1 << 19, 4096, 255, 4096);
    }

    fn access(&self, _req: &Request, _ino: INodeNo, _mask: fuser::AccessFlags, reply: ReplyEmpty) {
        reply.ok();
    }

    fn create(&self, _req: &Request, parent: INodeNo, name: &OsStr, _mode: u32, _umask: u32, _flags: i32, _kill_suid_gid: bool, _owner: fuser::Owner, reply: ReplyCreate) {
        let Some(path) = self.0.child(parent, name) else { return reply.error(Errno::ENOENT) };
        let local = self.0.cached(&path);
        if let Some(d) = local.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let Ok(file) = File::options().read(true).write(true).create(true).truncate(true).open(&local) else { return reply.error(Errno::EIO) };
        self.0.tree.write().unwrap().insert(&path, Node { dir: false, size: 0, mtime: now_secs() });
        let fh = self.0.new_handle(&path, file, true);
        match self.0.entry(&path) {
            Some((_, attr)) => reply.created(&TTL, &attr, Generation(0), FileHandle(fh), FopenFlags::empty()),
            None => reply.error(Errno::EIO),
        }
    }
}

// ── Mounting ──

/// FUSE is there to use: the device, and fusermount3 to mount without root.
pub fn available() -> bool {
    Path::new("/dev/fuse").exists() && ["/run/wrappers/bin/fusermount3", "/usr/bin/fusermount3", "/bin/fusermount3", "/usr/local/bin/fusermount3"].iter().any(|p| Path::new(p).exists())
}

fn fusermount() -> &'static str {
    ["/run/wrappers/bin/fusermount3", "/usr/bin/fusermount3", "/bin/fusermount3", "/usr/local/bin/fusermount3"].into_iter().find(|p| Path::new(p).exists()).unwrap_or("fusermount3")
}

/// A mount left behind by a service that died («Transport endpoint is not connected»).
pub fn unmount(path: &Path) {
    let _ = std::process::Command::new(fusermount())
        .args(["-u", "-z"])
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

pub fn mounted(path: &Path) -> bool {
    std::fs::read_to_string("/proc/self/mounts").is_ok_and(|m| m.lines().any(|l| l.split(' ').nth(1).is_some_and(|p| Path::new(&p.replace("\\040", " ")) == path) && l.contains("fuse")))
}

/// Mounts the folder; the returned session unmounts it when dropped.
pub fn mount(drive: Arc<Drive>) -> std::io::Result<fuser::BackgroundSession> {
    let path = drive.pair.local.clone();
    if mounted(&path) || std::fs::metadata(&path).is_err() {
        unmount(&path);
    }
    std::fs::create_dir_all(&path)?;
    let mut config = fuser::Config::default();
    config.mount_options.extend([MountOption::FSName("tpcloud".into()), MountOption::Subtype("tpcloud".into()), MountOption::DefaultPermissions, MountOption::NoAtime]);
    config.n_threads = Some(4);
    fuser::spawn_mount(DriveFs(drive), &path, &config)
}

/// The service's companion for a mounted folder: keeps the listing fresh, fetches what's kept
/// here, forgets what wasn't opened in a day, and sends what waited for the network.
pub fn keep_up(drive: Arc<Drive>, stop: Arc<std::sync::atomic::AtomicBool>) {
    std::thread::spawn(move || {
        let mut last = std::time::Instant::now() - REFRESH_EVERY;
        while !stop.load(Ordering::Relaxed) {
            if last.elapsed() >= REFRESH_EVERY || drive.poke.swap(false, Ordering::Relaxed) {
                last = std::time::Instant::now();
                if drive.refresh().is_ok() {
                    drive.retry_pending();
                    drive.housekeeping();
                }
                super::sync::record_demand_status(&drive.pair, drive.remote.lock().unwrap().is_some());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_moves_and_removes_whole_folders() {
        let mut t = Tree::from_listing(&[("a/b/c.txt".into(), 3, 1), ("a/d.txt".into(), 4, 2), ("e.txt".into(), 5, 3)], &["a".into(), "a/b".into()]);
        assert_eq!(t.children[""], BTreeSet::from(["a".to_string(), "e.txt".to_string()]));
        t.rename("a", "x/y");
        assert!(t.nodes.contains_key("x/y/b/c.txt") && t.nodes.contains_key("x/y/d.txt"));
        assert!(!t.nodes.contains_key("a") && !t.nodes.contains_key("a/b/c.txt"));
        assert_eq!(t.children[""], BTreeSet::from(["e.txt".to_string(), "x".to_string()]));
        t.remove("x");
        assert_eq!(t.nodes.len(), 2); // the root and e.txt
    }

    #[test]
    fn hidden_names_stay_here() {
        assert!(local_only(".file.swp") && local_only("docs/.~lock.odt#") && local_only(".git/config"));
        assert!(!local_only("docs/report.odt"));
        assert!(under("a/b", "a") && under("a", "a") && !under("ab", "a") && under("x", ""));
    }
}
