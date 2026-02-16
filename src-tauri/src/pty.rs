// ---------------------------------------------------------------------------
// PTY — Pseudo-terminal management for embedded Claude Code terminal
// ---------------------------------------------------------------------------
// Spawns Claude Code (or any command) in a PTY and streams output to the
// frontend via Tauri events. Frontend sends keystrokes back via commands.
// ---------------------------------------------------------------------------

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

struct PtySession {
    writer: Box<dyn Write + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    _command: String,
    running: Arc<std::sync::atomic::AtomicBool>,
}

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

static SESSIONS: std::sync::LazyLock<Mutex<HashMap<String, PtySession>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Spawn a command in a new PTY session.
/// Returns a session ID. Output is streamed via `pty:output` Tauri events.
pub fn spawn(
    app: AppHandle,
    command: Option<String>,
    cols: u16,
    rows: u16,
) -> Result<String, String> {
    let cmd = command.unwrap_or_else(|| "claude".to_string());
    let session_id = uuid::Uuid::new_v4().to_string();

    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Build the command
    let mut cmd_builder = CommandBuilder::new(&cmd);

    // Set up environment
    if let Ok(home) = std::env::var("HOME") {
        cmd_builder.env("HOME", &home);

        // Ensure PATH includes common binary locations
        let path = std::env::var("PATH").unwrap_or_default();
        let extra_paths = format!(
            "/usr/local/bin:{}/.local/bin:{}/.cargo/bin:{}",
            home, home, path
        );
        cmd_builder.env("PATH", extra_paths);
    }

    cmd_builder.env("TERM", "xterm-256color");
    cmd_builder.env("COLORTERM", "truecolor");

    // Spawn the child process
    let mut child = pair
        .slave
        .spawn_command(cmd_builder)
        .map_err(|e| format!("Failed to spawn '{}': {}", cmd, e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to get PTY reader: {}", e))?;

    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();
    let sid = session_id.clone();

    // Background thread: read PTY output and emit Tauri events
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app_clone.emit("pty:output", (&sid, &data));
                }
                Err(e) => {
                    eprintln!("PTY read error for {}: {}", sid, e);
                    break;
                }
            }
        }
        running_clone.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = app_clone.emit("pty:exit", &sid);
    });

    // Background thread: wait for child process exit
    let running_exit = running.clone();
    let sid_exit = session_id.clone();
    let app_exit = app;
    std::thread::spawn(move || {
        let _ = child.wait();
        running_exit.store(false, std::sync::atomic::Ordering::Relaxed);
        let _ = app_exit.emit("pty:exit", &sid_exit);
    });

    // Store session (we need the master for resize)
    let master = pair.master;
    let session = PtySession {
        writer,
        master,
        _command: cmd,
        running,
    };

    SESSIONS
        .lock()
        .map_err(|_| "Session lock poisoned".to_string())?
        .insert(session_id.clone(), session);

    Ok(session_id)
}

/// Write data (keystrokes) to a PTY session.
pub fn write_to(session_id: &str, data: &str) -> Result<(), String> {
    let mut sessions = SESSIONS
        .lock()
        .map_err(|_| "Session lock poisoned".to_string())?;

    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| format!("PTY session '{}' not found", session_id))?;

    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("PTY write error: {}", e))?;

    session
        .writer
        .flush()
        .map_err(|e| format!("PTY flush error: {}", e))?;

    Ok(())
}

/// Resize a PTY session.
pub fn resize(session_id: &str, cols: u16, rows: u16) -> Result<(), String> {
    let sessions = SESSIONS
        .lock()
        .map_err(|_| "Session lock poisoned".to_string())?;

    let session = sessions
        .get(session_id)
        .ok_or_else(|| format!("PTY session '{}' not found", session_id))?;

    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("PTY resize error: {}", e))?;

    Ok(())
}

/// Kill a PTY session and clean up.
pub fn kill(session_id: &str) -> Result<(), String> {
    let mut sessions = SESSIONS
        .lock()
        .map_err(|_| "Session lock poisoned".to_string())?;

    if let Some(session) = sessions.remove(session_id) {
        session
            .running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        // Dropping the session closes the PTY
        drop(session);
    }

    Ok(())
}
