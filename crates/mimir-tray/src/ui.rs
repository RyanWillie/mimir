use std::sync::{Arc, Mutex};

use mimir_tray::{MemoryClient, ServiceManager, DaemonStatus};
use mimir_core::{Config, Result};
use tauri::{Emitter, Manager, WebviewWindow, AppHandle};

// Logs streaming task handle holder
struct LogsState {
    handle: Option<tauri::async_runtime::JoinHandle<()>>,
}

impl Default for LogsState {
    fn default() -> Self {
        Self { handle: None }
    }
}

#[tauri::command]
async fn tray_get_status() -> std::result::Result<DaemonStatus, String> {
    let config = Config::load().unwrap_or_default();
    let client = MemoryClient::new(config).map_err(|e| e.to_string())?;
    client.get_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tray_start_daemon(password: Option<String>, full_features: Option<bool>) -> std::result::Result<(), String> {
    let config = Config::load().unwrap_or_default();
    let mut svc = ServiceManager::new(config).map_err(|e| e.to_string())?;
    // Default to full feature mode unless explicitly disabled by caller
    let full = full_features.unwrap_or(true);
    svc.start_daemon_with_options(password, full).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tray_stop_daemon() -> std::result::Result<(), String> {
    let config = Config::load().unwrap_or_default();
    let mut svc = ServiceManager::new(config).map_err(|e| e.to_string())?;
    svc.stop_daemon().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn tray_is_password_mode() -> std::result::Result<bool, String> {
    let config = Config::load().unwrap_or_default();
    Ok(config.use_password_encryption || config.encryption_mode.to_lowercase() == "password")
}

#[tauri::command]
async fn tray_start_logs(app_handle: AppHandle) -> std::result::Result<(), String> {
    // Access managed state through the app handle and clone the Arc so it can be moved
    let state_arc: Arc<Mutex<LogsState>> = app_handle.state::<Arc<Mutex<LogsState>>>().inner().clone();

    // If already streaming, do nothing
    let mut guard = state_arc.lock().unwrap();
    if guard.handle.is_some() {
        // Let the UI know it's already on
        let _ = app_handle.emit_to("main", "log_line", "Log streaming already active".to_string());
        return Ok(());
    }

    // Prefer SSE logs from the daemon; fall back to file tailing if unreachable
    let config = Config::load().unwrap_or_default();
    let base_url = format!("http://localhost:{}", config.server.port);
    let app_for_task = app_handle.clone();
    let _ = app_handle.emit_to("main", "log_line", "Starting log stream...".to_string());
    let handle = tauri::async_runtime::spawn(async move {
        use futures_util::StreamExt;
        use tokio::time::{sleep, Duration};

        // Simple SSE reader over reqwest with reconnect
        async fn stream_sse_logs(base_url: &str, app: &AppHandle) -> bool {
            let client = reqwest::Client::new();
            let url = format!("{}/logs", base_url);
            let msg = format!("[tray] Connecting to {} via SSE...", url);
            let _ = app.emit("log_line", msg.clone());
            if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", msg.clone()); }
            let resp = match client.get(&url).send().await {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("[tray] SSE connection error: {}", e);
                    let _ = app.emit("log_line", msg.clone());
                    if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", msg.clone()); }
                    return false;
                }
            };
            if !resp.status().is_success() {
                let msg = format!("[tray] SSE HTTP status: {}", resp.status());
                let _ = app.emit("log_line", msg.clone());
                if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", msg.clone()); }
                return false;
            }
            let mut buffer: Vec<u8> = Vec::with_capacity(8192);
            let mut stream = resp.bytes_stream();
            let mut last_data = std::time::Instant::now();
            let mut saw_payload = false;
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(bytes) => {
                        buffer.extend_from_slice(&bytes);
                        // SSE events are separated by a blank line (\n\n or \r\n\r\n)
                        loop {
                            let (maybe_pos, sep_len) = if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                                (Some(pos), 4)
                            } else if let Some(pos) = buffer.windows(2).position(|w| w == b"\n\n") {
                                (Some(pos), 2)
                            } else {
                                (None, 0)
                            };
                            if let Some(pos) = maybe_pos {
                                let event = buffer.drain(..pos + sep_len).collect::<Vec<u8>>();
                                // Parse data: lines
                                let text = String::from_utf8_lossy(&event);
                                let mut data_lines: Vec<String> = Vec::new();
                                for line in text.lines() {
                                    if line.starts_with("data:") {
                                        let rest = &line[5..];
                                        let rest = rest.strip_prefix(' ').unwrap_or(rest);
                                        data_lines.push(rest.to_string());
                                    }
                                }
                                if !data_lines.is_empty() {
                                    let payload = data_lines.join("\n");
                                    let _ = app.emit("log_line", payload.clone());
                                    if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", payload.clone()); }
                                    last_data = std::time::Instant::now();
                                    saw_payload = true;
                                }
                            } else {
                                break;
                            }
                        }
                        // If we connected but received no data for a while, fall back
                        if !saw_payload && last_data.elapsed() > Duration::from_secs(3) {
                            let _ = app.emit("log_line", "[tray] No SSE data, falling back to file tail...".to_string());
                            return false;
                        }
                    }
                    Err(e) => {
                        let msg = format!("[tray] SSE stream error: {}", e);
                        let _ = app.emit("log_line", msg.clone());
                        if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", msg.clone()); }
                        return false;
                    }
                }
            }
            true
        }

        // Fallback: tail log files if SSE is unavailable
        async fn tail_log_files(app: &AppHandle) {
            use std::path::PathBuf;
            use tokio::{fs, fs::File};
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            use tokio::time::{sleep, Duration, Instant};

            let log_dir = mimir_core::get_default_app_dir().join("logs");

            async fn latest_log_file(dir: &PathBuf) -> Option<PathBuf> {
                use std::time::SystemTime;
                let mut latest: Option<(SystemTime, PathBuf)> = None;
                let mut stack: Vec<PathBuf> = vec![dir.clone()];
                while let Some(current) = stack.pop() {
                    if let Ok(mut rd) = fs::read_dir(&current).await {
                        while let Ok(Some(entry)) = rd.next_entry().await {
                            let path = entry.path();
                            let Ok(meta) = entry.metadata().await else { continue };
                            if meta.is_dir() {
                                stack.push(path);
                            } else if path.extension().map(|e| e == "log").unwrap_or(false) {
                                if let Ok(modified) = meta.modified() {
                                    if latest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                                        latest = Some((modified, path));
                                    }
                                }
                            }
                        }
                    }
                }
                latest.map(|(_, p)| p)
            }

            let start_wait = Instant::now();
            while !log_dir.exists() && start_wait.elapsed() < Duration::from_secs(5) {
                sleep(Duration::from_millis(200)).await;
            }

            let mut current_path: Option<PathBuf> = latest_log_file(&log_dir).await;
            let start_wait2 = Instant::now();
            while current_path.is_none() && start_wait2.elapsed() < Duration::from_secs(10) {
                sleep(Duration::from_millis(200)).await;
                current_path = latest_log_file(&log_dir).await;
            }

            if current_path.is_none() {
                let msg = "No log file found yet. Start the daemon to generate logs.".to_string();
                let _ = app.emit("log_line", msg.clone());
                if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", msg.clone()); }
                return;
            }

            let mut path = current_path.unwrap();
            let Ok(mut file) = File::open(&path).await else { return };
            let _ = file.seek(std::io::SeekFrom::End(0)).await;
            let mut buf = vec![0u8; 8192];
            let mut idle_ticks = 0u32;
            loop {
                match file.read(&mut buf).await {
                    Ok(0) => {
                        idle_ticks += 1;
                        if idle_ticks % 10 == 0 {
                            if let Some(newest) = latest_log_file(&log_dir).await {
                                if newest != path {
                                    path = newest;
                                    if let Ok(newf) = File::open(&path).await {
                                        file = newf;
                                        let _ = file.seek(std::io::SeekFrom::End(0)).await;
                                    }
                                }
                            }
                        }
                        sleep(Duration::from_millis(300)).await;
                    }
                    Ok(n) => {
                        idle_ticks = 0;
                        let text = String::from_utf8_lossy(&buf[..n]).to_string();
                        for line in text.split('\n') {
                            if !line.trim().is_empty() {
                                let payload = line.trim().to_string();
                                let _ = app.emit("log_line", payload.clone());
                                if let Some(win) = app.get_webview_window("main") { let _ = win.emit("log_line", payload.clone()); }
                            }
                        }
                    }
                    Err(_) => {
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        }

        // Main loop: try SSE, reconnect on failure; fallback to tailing if SSE repeatedly fails
        let mut sse_failures = 0u32;
        loop {
            if stream_sse_logs(&base_url, &app_for_task).await {
                // Completed (server closed gracefully). Try reconnect after a short delay.
                sleep(Duration::from_millis(300)).await;
                continue;
            } else {
                sse_failures += 1;
                if sse_failures >= 3 {
                    // Fallback to file tailing
                    tail_log_files(&app_for_task).await;
                    // If file tailing returns, wait and retry SSE again
                    sse_failures = 0;
                    sleep(Duration::from_secs(1)).await;
                } else {
                    // Brief backoff then retry SSE
                    sleep(Duration::from_millis(400)).await;
                }
            }
        }
    });
    guard.handle = Some(handle);
    Ok(())
}

#[tauri::command]
async fn tray_stop_logs(app: AppHandle) -> std::result::Result<(), String> {
    let state_arc: Arc<Mutex<LogsState>> = app.state::<Arc<Mutex<LogsState>>>().inner().clone();
    let mut guard = state_arc.lock().unwrap();
    if let Some(handle) = guard.handle.take() {
        handle.abort();
    }
    Ok(())
}

#[tauri::command]
async fn tray_emit_test_log(app: AppHandle, msg: Option<String>) -> std::result::Result<(), String> {
    let text = msg.unwrap_or_else(|| "[tray] test emit".to_string());
    let _ = app.emit("log_line", text.clone());
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("log_line", format!("[win] {}", text));
    }
    Ok(())
}

pub fn run_tauri() -> Result<()> {
    let logs_state: Arc<Mutex<LogsState>> = Arc::new(Mutex::new(LogsState::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(logs_state)
        .on_window_event(|_window, _event| {})
        .invoke_handler(tauri::generate_handler![
            tray_get_status,
            tray_start_daemon,
            tray_stop_daemon,
            tray_is_password_mode,
            tray_start_logs,
            tray_stop_logs,
            tray_emit_test_log
        ])
        .setup(|app| {
            // Show main window on start
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.show();
                // Auto-start logs streaming
                // Defer log start to after window ready without capturing non-'static references
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Give the UI a moment to initialize
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    let _ = tray_start_logs(app_handle).await;
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| mimir_core::MimirError::ServerError(format!("Tauri error: {}", e)))?;

    Ok(())
}
