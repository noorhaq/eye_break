//! Run-on-startup toggle for portable / non-packaged installs.
//!
//! The `.deb` package already ships a `/etc/xdg/autostart/eye-break.desktop`
//! entry, so this module is only relevant to users running the raw binary
//! directly. It is **not** part of `Config` — it's a live filesystem
//! check/toggle, not a persisted setting, and a future tray menu item is
//! expected to call `is_enabled`/`set_enabled` directly rather than reading
//! it from config.

use std::io;
use std::path::PathBuf;

fn autostart_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config").join("autostart"))
}

fn autostart_file() -> Option<PathBuf> {
    Some(autostart_dir()?.join("eye-break.desktop"))
}

/// Whether a user-level autostart entry currently exists.
#[allow(dead_code)]
pub fn is_enabled() -> bool {
    autostart_file().map(|p| p.exists()).unwrap_or(false)
}

/// Create or remove `~/.config/autostart/eye-break.desktop`.
#[allow(dead_code)]
pub fn set_enabled(enabled: bool) -> io::Result<()> {
    let path = autostart_file().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "could not determine home directory")
    })?;

    if !enabled {
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        return Ok(());
    }

    if let Some(dir) = autostart_dir() {
        std::fs::create_dir_all(dir)?;
    }

    let exe = std::env::current_exe()?;
    let exec = exe.to_string_lossy().to_string();

    let contents = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Eye Break\n\
         Comment=Periodic eye-break reminders with guided exercises\n\
         Exec={exec}\n\
         Icon=eye-break\n\
         Terminal=false\n\
         Categories=Utility;Health;\n\
         StartupNotify=false\n\
         X-GNOME-Autostart-enabled=true\n"
    );

    std::fs::write(&path, contents)
}
