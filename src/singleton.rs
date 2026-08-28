//! Single-instance guards, so autostart + a manual launch (or repeatedly
//! clicking a tray menu item) can't end up with two of the same process
//! fighting over the same tray icon, scheduler state, or window.
//!
//! Uses an advisory `flock()` on a lock file per "kind" (tray/settings/
//! timer) rather than a PID file: a PID file can go stale if the process
//! died without cleaning up (leaving a live PID reused by something else),
//! but a `flock` is released automatically by the kernel the moment the
//! holding process exits for any reason, crash included — no stale state
//! possible. Each kind gets its own lock so, e.g., a second Settings window
//! request doesn't get blocked by the tray process's lock.

use std::fs::File;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

/// A held instance lock. Keep this alive for as long as this process should
/// count as "the" instance of its kind — dropping it (or the process
/// exiting) releases the lock immediately.
pub struct InstanceLock {
    _file: File,
}

fn lock_path(kind: &str) -> PathBuf {
    crate::paths::config_file(&format!("{kind}.lock"))
}

/// Tries to become the sole instance of `kind`. Returns `Some(lock)` if this
/// process is now (and stays, until the lock is dropped) the only one
/// holding it; `None` if another live process already holds it, in which
/// case the caller should back off (exit, or raise the existing window)
/// rather than proceed.
pub fn try_acquire(kind: &str) -> Option<InstanceLock> {
    let path = lock_path(kind);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .ok()?;

    let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret == 0 {
        Some(InstanceLock { _file: file })
    } else {
        // EWOULDBLOCK means another process holds it — the expected "not
        // the first instance" case. Any other errno we also treat as "don't
        // risk running twice", logging so it's not silently mysterious.
        if io::Error::last_os_error().raw_os_error() != Some(libc::EWOULDBLOCK) {
            eprintln!(
                "eye-break: could not acquire {kind} instance lock: {}",
                io::Error::last_os_error()
            );
        }
        None
    }
}
