use log::info;

#[cfg(target_os = "macos")]
fn schedule_restart_after_exit() -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    // Derive .app bundle path from the current executable
    // Exe is at: /Applications/推送姬.app/Contents/MacOS/推送姬
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {}", e))?;
    let app_bundle = exe
        .parent() // Contents/MacOS
        .and_then(|p| p.parent()) // Contents
        .and_then(|p| p.parent()) // 推送姬.app
        .ok_or_else(|| "Cannot locate app bundle".to_string())?;

    let parent_pid = std::process::id().to_string();
    let log_path = std::env::temp_dir().join("tuisongji-restart-helper.log");

    info!(
        "[updater] scheduling restart parent_pid={} app_bundle={} log={}",
        parent_pid,
        app_bundle.display(),
        log_path.display(),
    );

    let script = r#"
parent="$1"
app="$2"
log="$3"

timestamp() { date '+%Y-%m-%dT%H:%M:%S%z'; }
write_log() { printf '%s %s\n' "$(timestamp)" "$*" >> "$log"; }

write_log "restart helper started parent=$parent app=$app"

attempt=0
while kill -0 "$parent" 2>/dev/null; do
    if [ "$attempt" -ge 600 ]; then
        write_log "parent still alive after timeout attempts=$attempt"
        break
    fi
    attempt=$((attempt + 1))
    sleep 0.1
done

write_log "parent exited attempts=$attempt, opening app"
sleep 0.2
/usr/bin/open -n "$app"
status=$?
write_log "open finished status=$status"
exit "$status"
"#;

    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg("restart-helper")
        .arg(parent_pid)
        .arg(app_bundle)
        .arg(log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("failed to schedule restart: {}", e))?;

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn schedule_restart_after_exit() -> Result<(), String> {
    Err("restart after update is not supported on this platform".to_string())
}

#[tauri::command]
pub fn restart_application(app: tauri::AppHandle) -> Result<(), String> {
    schedule_restart_after_exit()?;
    app.exit(0);
    #[allow(unreachable_code)]
    Ok(())
}
