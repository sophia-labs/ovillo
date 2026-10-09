//! The link to a running Ovillo: its control socket, one JSON line per
//! request and per reply. The connection is kept open (so the CTL
//! annunciator stays lit while a Claude is attached) and remade once if the
//! app has restarted.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

/// The per-user temporary directory: Darwin's `confstr(_CS_DARWIN_USER_TEMP_DIR)`,
/// which the app uses too, so both agree even when `$TMPDIR` differs.
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

/// `--socket <path>`, else `$OVILLO_SOCKET`, else `ovillo.sock` in the user temp dir.
pub fn socket_path(arg: Option<&str>) -> PathBuf {
    arg.map(PathBuf::from)
        .or_else(|| std::env::var("OVILLO_SOCKET").ok().filter(|s| !s.is_empty()).map(PathBuf::from))
        .unwrap_or_else(|| user_temp_dir().join("ovillo.sock"))
}

pub enum LinkError {
    /// Nobody answers on the socket.
    NotRunning(String),
    /// The instrument answered with an error.
    Refused(String),
}

impl LinkError {
    pub fn text(&self) -> String {
        match self {
            LinkError::NotRunning(s) | LinkError::Refused(s) => s.clone(),
        }
    }
}

pub struct Link {
    pub path: PathBuf,
    conn: Option<(BufReader<UnixStream>, UnixStream)>,
    next_id: u64,
}

impl Link {
    pub fn new(path: PathBuf) -> Self {
        Self { path, conn: None, next_id: 1 }
    }

    fn connect(&mut self) -> Result<(), LinkError> {
        if self.conn.is_some() {
            return Ok(());
        }
        let s = UnixStream::connect(&self.path).map_err(|e| {
            LinkError::NotRunning(format!(
                "Ovillo is not running on {} ({e}). Start it from the ovillo/ workspace with `cargo run -p ovillo-app` \
                 (add `-- --socket {}` if this is not the default path), then try again.",
                self.path.display(),
                self.path.display()
            ))
        })?;
        let _ = s.set_read_timeout(Some(Duration::from_secs(30)));
        let r = s.try_clone().map_err(|e| LinkError::NotRunning(e.to_string()))?;
        self.conn = Some((BufReader::new(r), s));
        Ok(())
    }

    /// Ask the instrument to do `op`; its result, or why not.
    pub fn call(&mut self, op: &str, args: Value) -> Result<Value, LinkError> {
        let id = self.next_id;
        self.next_id += 1;
        let line = json!({"id": id, "op": op, "args": args}).to_string();
        let mut last = String::new();
        for _attempt in 0..2 {
            self.connect()?;
            let Some((reader, writer)) = self.conn.as_mut() else { continue };
            if let Err(e) = writeln!(writer, "{line}").and_then(|()| writer.flush()) {
                // A connection to an instrument that has gone: reconnect once.
                last = e.to_string();
                self.conn = None;
                continue;
            }
            let mut reply = String::new();
            match reader.read_line(&mut reply) {
                Ok(0) => {
                    last = "the instrument closed the connection".into();
                    self.conn = None;
                }
                Ok(_) => {
                    let v: Value = serde_json::from_str(&reply)
                        .map_err(|e| LinkError::Refused(format!("the instrument sent something that is not JSON: {e}")))?;
                    if v.get("ok") == Some(&Value::Bool(true)) {
                        return Ok(v.get("result").cloned().unwrap_or(Value::Null));
                    }
                    let err = v.get("error").and_then(Value::as_str).unwrap_or("unknown error");
                    return Err(LinkError::Refused(err.to_string()));
                }
                Err(e) if matches!(e.kind(), std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe) => {
                    last = e.to_string();
                    self.conn = None;
                }
                Err(e) => {
                    // A timeout: the request may have been applied, so do not send it again.
                    self.conn = None;
                    return Err(LinkError::NotRunning(format!("no answer from the instrument on {}: {e}", self.path.display())));
                }
            }
        }
        Err(LinkError::NotRunning(format!("lost the instrument on {}: {last}", self.path.display())))
    }
}
