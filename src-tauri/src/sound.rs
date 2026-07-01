use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use std::sync::OnceLock;

enum AudioCmd {
    Play { path: PathBuf, volume: f32 },
    Stop,
}

static TX: OnceLock<Mutex<Sender<AudioCmd>>> = OnceLock::new();

fn ensure_audio_thread() -> &'static Mutex<Sender<AudioCmd>> {
    TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<AudioCmd>();
        std::thread::spawn(move || {
            let (stream, stream_handle) =
                OutputStream::try_default().expect("Failed to initialize audio output");
            let sink = Sink::try_new(&stream_handle).expect("Failed to create audio sink");
            // Keep stream alive
            let _stream = stream;

            for cmd in rx {
                match cmd {
                    AudioCmd::Play { path, volume } => {
                        sink.clear();
                        sink.set_volume(volume.clamp(0.0, 1.0));
                        if let Ok(file) = fs::File::open(&path) {
                            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                                sink.append(source);
                            }
                        }
                    }
                    AudioCmd::Stop => {
                        sink.clear();
                    }
                }
            }
        });
        Mutex::new(tx)
    })
}

/// Play a specific sound file with the given volume (0.0 - 1.0).
pub fn play_file(path: &Path, volume: f32) -> Result<(), String> {
    let tx = ensure_audio_thread()
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    tx.send(AudioCmd::Play {
        path: path.to_path_buf(),
        volume,
    })
    .map_err(|e| format!("Send error: {}", e))
}

/// Stop any currently playing sound.
#[allow(dead_code)]
pub fn stop() {
    if let Some(tx_lock) = TX.get() {
        if let Ok(tx) = tx_lock.lock() {
            let _ = tx.send(AudioCmd::Stop);
        }
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

/// Count downloaded sound files for a streamer.
/// Returns `(live_count, offline_count)`.
pub fn count_downloaded(name: &str, data_dir: &Path) -> (u32, u32) {
    let base = data_dir.join("sounds").join(name);
    let live_count = count_files_in_dir(&base.join("live"));
    let offline_count = count_files_in_dir(&base.join("offline"));
    (live_count, offline_count)
}

fn count_files_in_dir(dir: &Path) -> u32 {
    match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .count() as u32,
        Err(_) => 0,
    }
}
