//! The control socket: a Unix domain socket speaking JSON lines, so a Claude
//! session (through the `ovillo-mcp` server) can drive the running
//! instrument.
//!
//! ```text
//! → {"id": 1, "op": "get_state", "args": {}}
//! ← {"id": 1, "ok": true, "result": {...}}
//! ← {"id": 2, "ok": false, "error": "unknown parameter \"drve\""}
//! ```
//!
//! A listener thread accepts connections; each connection's thread forwards
//! its requests, each with a reply channel, over an mpsc channel to the UI.
//! The UI applies them on its timer (and every frame) through the same code
//! paths the keys use, so the screen, the menus, the annunciators and
//! `live.json` stay one story, and answers.
//!
//! The socket's path is `--socket <path>`, else `$OVILLO_SOCKET`, else
//! `ovillo.sock` in the per-user temporary directory (Darwin's
//! `confstr(_CS_DARWIN_USER_TEMP_DIR)`, which does not depend on `$TMPDIR`,
//! so a sandboxed MCP server and a terminal-launched app agree). A path
//! another live instance answers on is left alone (CTL OFF); a stale file
//! nobody answers on is removed and bound.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Longest Unix socket path macOS accepts (`sun_path` is 104 bytes with its NUL).
const MAX_PATH: usize = 103;

/// The per-user temporary directory.
pub fn user_temp_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn confstr(name: i32, buf: *mut u8, len: usize) -> usize;
        }
        const CS_DARWIN_USER_TEMP_DIR: i32 = 65537;
        let mut buf = [0u8; 1024];
        // SAFETY: confstr writes at most `len` bytes, NUL included, and returns the length it needs.
        let n = unsafe { confstr(CS_DARWIN_USER_TEMP_DIR, buf.as_mut_ptr(), buf.len()) };
        if n > 1 && n <= buf.len() {
            if let Ok(s) = std::str::from_utf8(&buf[..n - 1]) {
                return PathBuf::from(s);
            }
        }
    }
    std::env::var_os("TMPDIR").map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
}

/// Where the socket is, from the command line, the environment, or the default.
pub fn socket_path(arg: Option<&str>) -> Option<PathBuf> {
    let chosen = arg.map(str::to_string).or_else(|| std::env::var("OVILLO_SOCKET").ok().filter(|s| !s.is_empty()));
    match chosen.as_deref() {
        Some("off" | "none") => None,
        Some(p) => Some(PathBuf::from(p)),
        None => Some(user_temp_dir().join("ovillo.sock")),
    }
}

pub struct Request {
    pub op: String,
    pub args: Value,
    reply: mpsc::SyncSender<Result<Value, String>>,
}

impl Request {
    pub fn answer(self, r: Result<Value, String>) {
        let _ = self.reply.send(r);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Listening,
    /// Another live instance answers on the path.
    InUse,
    /// Disabled, or the socket could not be made.
    Off(String),
}

pub struct Control {
    pub path: Option<PathBuf>,
    pub status: Status,
    rx: Option<mpsc::Receiver<Request>>,
    clients: Arc<AtomicUsize>,
    last_activity_ms: Arc<AtomicU64>,
    pub served: u64,
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl Control {
    /// Bind the socket (or report why not) and start listening.
    pub fn start(path: Option<PathBuf>) -> Self {
        let clients = Arc::new(AtomicUsize::new(0));
        let last = Arc::new(AtomicU64::new(0));
        let mut c = Self { path: path.clone(), status: Status::Off("disabled".into()), rx: None, clients, last_activity_ms: last, served: 0 };
        let Some(path) = path else {
            eprintln!("ovillo: control socket disabled");
            return c;
        };
        if path.as_os_str().len() > MAX_PATH {
            c.status = Status::Off(format!("socket path is {} bytes; macOS allows {MAX_PATH}", path.as_os_str().len()));
            eprintln!("ovillo: control socket off: {:?}", c.status);
            return c;
        }
        if path.exists() {
            if UnixStream::connect(&path).is_ok() {
                c.status = Status::InUse;
                eprintln!("ovillo: another Ovillo answers on {}; control socket off (pass --socket <path> to use another)", path.display());
                return c;
            }
            // Nobody answers: a stale socket from an instance that did not clean up.
            let _ = std::fs::remove_file(&path);
        }
        let listener = match UnixListener::bind(&path) {
            Ok(l) => l,
            Err(e) => {
                c.status = Status::Off(format!("{}: {e}", path.display()));
                eprintln!("ovillo: control socket off: {e}");
                return c;
            }
        };
        let (tx, rx) = mpsc::channel();
        let clients = c.clients.clone();
        let last = c.last_activity_ms.clone();
        let spawned = std::thread::Builder::new().name("ovillo-control".into()).spawn(move || {
            for conn in listener.incoming() {
                let Ok(stream) = conn else { continue };
                let (tx, clients, last) = (tx.clone(), clients.clone(), last.clone());
                let _ = std::thread::Builder::new().name("ovillo-control-conn".into()).spawn(move || serve(stream, &tx, &clients, &last));
            }
        });
        if let Err(e) = spawned {
            c.status = Status::Off(e.to_string());
            return c;
        }
        eprintln!("ovillo: control socket {}", path.display());
        c.status = Status::Listening;
        c.rx = Some(rx);
        c
    }

    /// Requests waiting for the UI.
    pub fn pending(&mut self) -> Vec<Request> {
        let Some(rx) = &self.rx else { return Vec::new() };
        let mut v = Vec::new();
        while let Ok(r) = rx.try_recv() {
            v.push(r);
        }
        self.served += v.len() as u64;
        v
    }

    pub fn clients(&self) -> usize {
        self.clients.load(Ordering::Relaxed)
    }

    /// A client is connected, or one spoke in the last two seconds.
    pub fn active(&self) -> bool {
        self.clients() > 0 || now_ms().saturating_sub(self.last_activity_ms.load(Ordering::Relaxed)) < 2_000
    }

    pub fn describe(&self) -> Value {
        json!({
            "path": self.path.as_ref().map(|p| p.display().to_string()),
            "status": match &self.status {
                Status::Listening => "listening".to_string(),
                Status::InUse => "in use by another Ovillo".to_string(),
                Status::Off(why) => format!("off: {why}"),
            },
            "clients": self.clients(),
            "requests_served": self.served,
        })
    }
}

/// Remove `p` if it is a socket (never a regular file someone pointed us at).
pub fn remove_if_socket(p: &Path) {
    use std::os::unix::fs::FileTypeExt;
    if std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_socket()) {
        let _ = std::fs::remove_file(p);
    }
}

/// One connection: a request per line, a reply per line, in order.
fn serve(stream: UnixStream, tx: &mpsc::Sender<Request>, clients: &AtomicUsize, last: &AtomicU64) {
    clients.fetch_add(1, Ordering::Relaxed);
    last.store(now_ms(), Ordering::Relaxed);
    if let Ok(read) = stream.try_clone() {
        let mut writer = stream;
        for line in BufReader::new(read).lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            last.store(now_ms(), Ordering::Relaxed);
            let reply = match serde_json::from_str::<Value>(&line) {
                Err(e) => json!({"id": null, "ok": false, "error": format!("not JSON: {e}")}),
                Ok(v) => {
                    let id = v.get("id").cloned().unwrap_or(Value::Null);
                    match v.get("op").and_then(Value::as_str) {
                        None => json!({"id": id, "ok": false, "error": "missing \"op\""}),
                        Some(op) => {
                            let args = v.get("args").cloned().unwrap_or_else(|| json!({}));
                            let (rtx, rrx) = mpsc::sync_channel(1);
                            if tx.send(Request { op: op.to_string(), args, reply: rtx }).is_err() {
                                break;
                            }
                            match rrx.recv_timeout(Duration::from_secs(20)) {
                                Ok(Ok(result)) => json!({"id": id, "ok": true, "result": result}),
                                Ok(Err(e)) => json!({"id": id, "ok": false, "error": e}),
                                Err(_) => json!({"id": id, "ok": false, "error": "the instrument did not answer within 20 s"}),
                            }
                        }
                    }
                }
            };
            last.store(now_ms(), Ordering::Relaxed);
            if writeln!(writer, "{reply}").and_then(|()| writer.flush()).is_err() {
                break;
            }
        }
    }
    clients.fetch_sub(1, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_sock(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("ovillo-ctl-{tag}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn a_request_goes_to_the_ui_and_its_answer_comes_back() {
        let path = temp_sock("rt");
        let mut c = Control::start(Some(path.clone()));
        assert_eq!(c.status, Status::Listening);
        let stream = UnixStream::connect(&path).unwrap();
        let mut w = stream.try_clone().unwrap();
        writeln!(w, r#"{{"id": 7, "op": "echo", "args": {{"x": 1}}}}"#).unwrap();
        writeln!(w, "not json").unwrap();
        writeln!(w, r#"{{"id": 8}}"#).unwrap();
        // The UI's side: wait for the request, answer it.
        let mut got = Vec::new();
        for _ in 0..200 {
            got = c.pending();
            if !got.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let r = got.pop().expect("the request reached the UI");
        assert_eq!((r.op.as_str(), r.args.clone()), ("echo", json!({"x": 1})));
        r.answer(Ok(json!({"echoed": true})));
        assert!(c.active());
        let mut lines = BufReader::new(stream).lines();
        let first: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert_eq!(first, json!({"id": 7, "ok": true, "result": {"echoed": true}}));
        let second: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert_eq!(second["ok"], false);
        assert!(second["error"].as_str().unwrap().starts_with("not JSON"));
        let third: Value = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert_eq!(third, json!({"id": 8, "ok": false, "error": "missing \"op\""}));
        remove_if_socket(&path);
    }

    #[test]
    fn a_live_socket_is_left_alone_and_a_stale_one_is_replaced() {
        let path = temp_sock("own");
        let first = Control::start(Some(path.clone()));
        assert_eq!(first.status, Status::Listening);
        let second = Control::start(Some(path.clone()));
        assert_eq!(second.status, Status::InUse, "a second instance must not steal a live socket");
        remove_if_socket(&path);

        // A socket file nobody listens on any more.
        let stale = temp_sock("stale");
        drop(UnixListener::bind(&stale).unwrap());
        assert!(stale.exists());
        let c = Control::start(Some(stale.clone()));
        assert_eq!(c.status, Status::Listening, "a stale socket is replaced");
        assert!(UnixStream::connect(&stale).is_ok());
        remove_if_socket(&stale);

        let off = Control::start(None);
        assert!(matches!(off.status, Status::Off(_)));
        let long = PathBuf::from(format!("/tmp/{}.sock", "x".repeat(120)));
        assert!(matches!(Control::start(Some(long)).status, Status::Off(why) if why.contains("bytes")));
    }

    #[test]
    fn regular_files_are_never_removed_as_sockets() {
        let p = std::env::temp_dir().join(format!("ovillo-ctl-file-{}.txt", std::process::id()));
        std::fs::write(&p, "keep me").unwrap();
        remove_if_socket(&p);
        assert!(p.exists());
        let _ = std::fs::remove_file(&p);
    }
}
