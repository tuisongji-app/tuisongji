use rand::seq::SliceRandom;
use rodio::{Decoder, OutputStream, Sink};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Write};
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

fn validate_component(value: &str) -> Result<(), String> {
    if value.is_empty() || value == "." || value == ".." || value.contains(['/', '\\', ':', '\0']) {
        return Err("音效名称不能包含路径分隔符".to_string());
    }
    Ok(())
}

pub fn streamer_sounds_dir(name: &str, data_dir: &Path) -> Result<PathBuf, String> {
    validate_component(name)?;
    Ok(data_dir.join("sounds").join(name))
}

pub fn event_sounds_dir(name: &str, event_type: &str, data_dir: &Path) -> Result<PathBuf, String> {
    if !matches!(event_type, "live" | "offline") {
        return Err("无效的音效类型".to_string());
    }
    Ok(streamer_sounds_dir(name, data_dir)?.join(event_type))
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "wav" | "ogg" | "flac"
            )
        })
}

/// Copy a validated local audio file into the streamer's event directory.
/// Keep existing files intact by assigning a suffix when the name is already in use.
pub fn import_file(
    name: &str,
    event_type: &str,
    filename: &str,
    data: &[u8],
    data_dir: &Path,
) -> Result<(), String> {
    let dir = event_sounds_dir(name, event_type, data_dir)?;
    validate_component(filename)?;
    let path = Path::new(filename);
    if !is_audio_file(path) {
        return Err("请选择 MP3、WAV、OGG 或 FLAC 音频文件".to_string());
    }
    Decoder::new(Cursor::new(data.to_vec()))
        .map_err(|_| format!("无法读取音频文件：{}", filename))?;

    fs::create_dir_all(&dir).map_err(|e| format!("创建音效文件夹失败：{}", e))?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("无效的文件名")?;
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or("无效的扩展名")?;
    let mut index = 0u32;
    loop {
        let target = dir.join(if index == 0 {
            filename.to_string()
        } else {
            format!("{} ({}).{}", stem, index, extension)
        });
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut file) => {
                if let Err(e) = file.write_all(data) {
                    drop(file);
                    let _ = fs::remove_file(&target);
                    return Err(format!("保存音效失败：{}", e));
                }
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => index += 1,
            Err(e) => return Err(format!("保存音效失败：{}", e)),
        }
    }
}

pub fn sound_file_path(
    name: &str,
    event_type: &str,
    filename: &str,
    data_dir: &Path,
) -> Result<PathBuf, String> {
    validate_component(filename)?;
    Ok(event_sounds_dir(name, event_type, data_dir)?.join(filename))
}

/// Pick a random sound file from `{data_dir}/sounds/{name}/{event_type}/` and play it.
/// `event_type` is either `"live"` or `"offline"`.
pub fn play_random_for_streamer(
    name: &str,
    event_type: &str,
    volume: f32,
    data_dir: &Path,
) -> Result<(), String> {
    let dir = event_sounds_dir(name, event_type, data_dir)?;

    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_audio_file(&path) {
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
    let Ok(base) = streamer_sounds_dir(name, data_dir) else {
        return (vec![], vec![]);
    };
    (
        list_files_in_dir(&base.join("live")),
        list_files_in_dir(&base.join("offline")),
    )
}

fn list_files_in_dir(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && is_audio_file(&e.path()))
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .collect(),
        Err(_) => vec![],
    };
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "tuisongji-sounds-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn wav() -> Vec<u8> {
        // One silent sample: mono PCM, 16 bits, 8 kHz.
        let mut bytes = b"RIFF".to_vec();
        bytes.extend_from_slice(&38u32.to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&8000u32.to_le_bytes());
        bytes.extend_from_slice(&16000u32.to_le_bytes());
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&0i16.to_le_bytes());
        bytes
    }

    #[test]
    fn imports_local_sounds_for_each_event_without_remote_metadata() {
        let dir = TestDir::new();
        let audio = wav();
        import_file("本地主播", "live", "开播.WAV", &audio, &dir.0).unwrap();
        import_file("本地主播", "offline", "下播.wav", &audio, &dir.0).unwrap();

        let (live, offline) = list_downloaded_files("本地主播", &dir.0);
        assert_eq!(live, vec!["开播.WAV"]);
        assert_eq!(offline, vec!["下播.wav"]);
        assert_eq!(
            fs::read(sound_file_path("本地主播", "live", "开播.WAV", &dir.0).unwrap()).unwrap(),
            audio
        );
    }

    #[test]
    fn imports_preserve_existing_files_and_rename_collisions() {
        let dir = TestDir::new();
        let live_dir = event_sounds_dir("主播", "live", &dir.0).unwrap();
        fs::create_dir_all(&live_dir).unwrap();
        fs::write(live_dir.join("hello.wav"), b"existing file").unwrap();

        for _ in 0..2 {
            import_file("主播", "live", "hello.wav", &wav(), &dir.0).unwrap();
        }

        assert_eq!(
            fs::read(live_dir.join("hello.wav")).unwrap(),
            b"existing file"
        );
        assert_eq!(fs::read(live_dir.join("hello (1).wav")).unwrap(), wav());
        assert_eq!(fs::read(live_dir.join("hello (2).wav")).unwrap(), wav());
    }

    #[test]
    fn rejects_invalid_audio_and_paths_before_writing_files() {
        let dir = TestDir::new();
        for (name, event_type, filename, data) in [
            ("主播", "live", "invalid.wav", b"not audio".to_vec()),
            ("主播", "live", "audio.txt", wav()),
            ("../主播", "live", "audio.wav", wav()),
            ("主播", "../live", "audio.wav", wav()),
            ("主播", "live", "../audio.wav", wav()),
            ("主播", "live", "C:\\audio.wav", wav()),
        ] {
            assert!(import_file(name, event_type, filename, &data, &dir.0).is_err());
        }
        assert!(!dir.0.join("sounds").exists());
    }

    #[test]
    fn lists_supported_audio_only_and_reflects_manual_changes() {
        let dir = TestDir::new();
        let live_dir = event_sounds_dir("主播", "live", &dir.0).unwrap();
        fs::create_dir_all(live_dir.join("folder.wav")).unwrap();
        for filename in [
            "b.mp3",
            "a.FLAC",
            "c.ogg",
            "d.wav",
            ".DS_Store",
            "notes.txt",
        ] {
            fs::write(live_dir.join(filename), b"manually added").unwrap();
        }
        assert_eq!(
            list_downloaded_files("主播", &dir.0),
            (
                vec![
                    "a.FLAC".into(),
                    "b.mp3".into(),
                    "c.ogg".into(),
                    "d.wav".into()
                ],
                vec![]
            )
        );
        fs::remove_file(live_dir.join("b.mp3")).unwrap();
        assert_eq!(list_downloaded_files("主播", &dir.0).0.len(), 3);
    }
}
