use serde::{Deserialize, Serialize};

/// Which notification sound to play when a break overlay is triggered.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum SoundChoice {
    /// No sound.
    None,
    /// Built-in short chime.
    Chime,
    /// Built-in short bell.
    Bell,
    /// User-provided sound file, given by path.
    Custom(String),
}

impl Default for SoundChoice {
    fn default() -> Self {
        SoundChoice::Chime
    }
}

const CHIME_WAV: &[u8] = include_bytes!("../assets/sounds/chime.wav");
const BELL_WAV: &[u8] = include_bytes!("../assets/sounds/bell.wav");

/// Play the selected notification sound asynchronously on a background
/// thread. No-ops on `SoundChoice::None`. Any playback error (no player
/// found, broken/missing custom file, ...) is logged to stderr and never
/// panics or blocks the caller.
///
/// Implementation note: rather than linking an audio library (rodio/cpal
/// pull in ALSA's dev headers at build time, which aren't guaranteed to be
/// present), this shells out to whatever CLI player is on PATH — the same
/// approach the rest of the app already uses for `xrandr`/`xdotool`. Built-in
/// sounds are embedded and written to a temp file since players need a path.
pub fn play(choice: &SoundChoice) {
    let choice = choice.clone();
    if choice == SoundChoice::None {
        return;
    }
    std::thread::spawn(move || {
        if let Err(e) = play_blocking(&choice) {
            eprintln!("eye-break: failed to play notification sound: {e}");
        }
    });
}

fn play_blocking(choice: &SoundChoice) -> Result<(), String> {
    let path: std::path::PathBuf = match choice {
        SoundChoice::None => return Ok(()),
        SoundChoice::Chime => write_temp_wav("eye-break-chime", CHIME_WAV)?,
        SoundChoice::Bell => write_temp_wav("eye-break-bell", BELL_WAV)?,
        SoundChoice::Custom(path) => {
            let p = std::path::PathBuf::from(path);
            if !p.exists() {
                return Err(format!("custom sound file not found: {path}"));
            }
            p
        }
    };

    run_first_available_player(&path)
}

fn write_temp_wav(name: &str, bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    let path = std::env::temp_dir().join(format!("{name}.wav"));
    if !path.exists() {
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

/// Tries a handful of common CLI players in order, using whichever is
/// installed. `paplay`/`pw-play` cover the common PulseAudio/PipeWire
/// desktop case; `aplay` covers bare ALSA; `ffplay` is a broad fallback.
fn run_first_available_player(path: &std::path::Path) -> Result<(), String> {
    let candidates: &[(&str, &[&str])] = &[
        ("paplay", &[]),
        ("pw-play", &[]),
        ("aplay", &["-q"]),
        ("ffplay", &["-nodisp", "-autoexit", "-loglevel", "quiet"]),
    ];

    for (bin, extra_args) in candidates {
        let status = std::process::Command::new(bin)
            .args(*extra_args)
            .arg(path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => return Ok(()),
            Ok(_) => continue,  // player exists but failed on this file; try the next
            Err(_) => continue, // not installed; try the next
        }
    }

    Err("no audio player found (tried paplay, pw-play, aplay, ffplay)".to_string())
}
