use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread;

enum AudioCmd {
    Play { path: PathBuf, volume: f32 },
    Stop,
}

static TX: OnceLock<Mutex<Sender<AudioCmd>>> = OnceLock::new();

/// Initialize audio on the main thread. Must be called once during app setup.
pub fn init_audio() {
    let (stream, stream_handle) =
        OutputStream::try_default().expect("Failed to initialize audio output");

    // OutputStream must stay on the main thread (not Send on macOS)
    // Leak it to keep it alive for the lifetime of the app.
    let _ = Box::leak(Box::new(stream));

    let (tx, rx) = mpsc::channel::<AudioCmd>();
    TX.set(Mutex::new(tx))
        .ok()
        .expect("Audio already initialized");

    thread::spawn(move || {
        // stream_handle is Send, safe to use here
        let mut current: Option<Sink> = None;

        for cmd in rx {
            match cmd {
                AudioCmd::Play { path, volume } => {
                    if let Some(old) = current.take() {
                        old.clear();
                    }
                    let sink = match Sink::try_new(&stream_handle) {
                        Ok(s) => s,
                        Err(e) => {
                            log::warn!("Failed to create sink: {}", e);
                            continue;
                        }
                    };
                    sink.set_volume(volume.clamp(0.0, 1.0));
                    let bytes = match fs::read(&path) {
                        Ok(b) => b,
                        Err(e) => {
                            log::warn!("Failed to read {:?}: {}", path, e);
                            continue;
                        }
                    };
                    match Decoder::new(Cursor::new(bytes)) {
                        Ok(source) => {
                            sink.append(source);
                            log::info!("Playing: {:?}", path);
                            current = Some(sink);
                        }
                        Err(e) => log::warn!("Failed to decode {:?}: {}", path, e),
                    }
                }
                AudioCmd::Stop => {
                    if let Some(sink) = current.take() {
                        sink.clear();
                    }
                }
            }
        }
    });
}

fn sender() -> Result<std::sync::MutexGuard<'static, Sender<AudioCmd>>, String> {
    TX.get()
        .ok_or("Audio not initialized")?
        .lock()
        .map_err(|e| format!("Lock error: {}", e))
}

/// Play a specific sound file with the given volume (0.0 - 1.0).
pub fn play_file(path: &Path, volume: f32) -> Result<(), String> {
    sender()?
        .send(AudioCmd::Play {
            path: path.to_path_buf(),
            volume,
        })
        .map_err(|e| format!("Send error: {}", e))
}

/// Stop any currently playing sound.
#[allow(dead_code)]
pub fn stop() {
    if let Ok(tx) = sender() {
        let _ = tx.send(AudioCmd::Stop);
    }
}

/// Pick a random sound file from `{data_dir}/sounds/{name}/{event_type}/` and play it.
/// `event_type` is either `"live"` or `"offline"`.
pub fn play_random_for_streamer(
    name: &str,
    event_type: &str,
    volume: f32,
    data_dir: &Path,
) -> Result<(), String> {
    let dir = data_dir.join("sounds").join(name).join(event_type);

    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }

    if files.is_empty() {
        return Err(format!("No sound files in {}", dir.display()));
    }

    let chosen = files
        .choose(&mut rand::thread_rng())
        .ok_or("Failed to pick random file")?;

    play_file(chosen, volume)
}

/// List downloaded file names for a streamer.
/// Returns `(live_files, offline_files)`.
pub fn list_downloaded_files(name: &str, data_dir: &Path) -> (Vec<String>, Vec<String>) {
    let base = data_dir.join("sounds").join(name);
    (
        list_files_in_dir(&base.join("live")),
        list_files_in_dir(&base.join("offline")),
    )
}

fn list_files_in_dir(dir: &Path) -> Vec<String> {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .collect(),
        Err(_) => vec![],
    }
}
