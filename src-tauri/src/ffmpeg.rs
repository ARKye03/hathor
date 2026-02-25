mod args;
mod ops;
mod probe;
mod progress;

use std::fs;
use std::io::{BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use self::args::{build_args, build_compress_two_pass_args, TwoPassArgs};
use self::ops::FfmpegOperation;
use self::probe::{expand_media_inputs as expand_media_inputs_inner, probe_media_inner, MediaInfo};
use self::progress::parse_progress;

pub struct FfmpegState {
    running: Arc<Mutex<Option<RunningProcess>>>,
}

impl FfmpegState {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for FfmpegState {
    fn default() -> Self {
        Self::new()
    }
}

struct RunningProcess {
    child: Child,
    output_path: Option<String>,
    cleanup_partial: bool,
    temp_files: Vec<String>,
}

fn lock_running<'a>(
    running: &'a Arc<Mutex<Option<RunningProcess>>>,
) -> std::sync::MutexGuard<'a, Option<RunningProcess>> {
    running.lock().unwrap_or_else(|poisoned| {
        eprintln!("recovering from poisoned ffmpeg running-process mutex");
        poisoned.into_inner()
    })
}

fn shell_escape(arg: &str) -> String {
    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_./:=".contains(c))
    {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

fn cleanup_partial_output(path: &str) {
    if let Err(e) = fs::remove_file(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("failed to cleanup partial output '{}': {}", path, e);
        }
    }
}

fn cleanup_temp_files(paths: &[String]) {
    for p in paths {
        if let Err(e) = fs::remove_file(p) {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("failed to cleanup temp file '{}': {}", p, e);
            }
        }
    }
}

async fn run_ffmpeg_once(
    app: &AppHandle,
    running: Arc<Mutex<Option<RunningProcess>>>,
    args: Vec<String>,
    output_path: Option<String>,
    cleanup_partial: bool,
    temp_files: Vec<String>,
    cleanup_temp_on_exit: bool,
) -> Result<i32, String> {
    let command = format!(
        "ffmpeg {}",
        args.iter()
            .map(|a| shell_escape(a))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let _ = app.emit("ffmpeg://command", command);

    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stderr = child
        .stderr
        .take()
        .ok_or("Failed to capture ffmpeg stderr")?;

    {
        let mut guard = lock_running(&running);
        if let Some(ref mut prev) = *guard {
            let _ = prev.child.kill();
            cleanup_temp_files(&prev.temp_files);
        }
        *guard = Some(RunningProcess {
            child,
            output_path,
            cleanup_partial,
            temp_files,
        });
    }

    let child_arc = Arc::clone(&running);
    let app_clone = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut reader = BufReader::new(stderr);
        let mut pending = String::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                    while let Some(pos) = pending.find(['\r', '\n']) {
                        let line = pending[..pos].trim().to_string();
                        pending = pending[pos + 1..].to_string();
                        if line.is_empty() {
                            continue;
                        }
                        if let Some(progress) = parse_progress(&line) {
                            let _ = app_clone.emit("ffmpeg://progress", progress);
                        } else {
                            let _ = app_clone.emit("ffmpeg://log", line);
                        }
                    }
                }
            }
        }
        let line = pending.trim().to_string();
        if !line.is_empty() {
            if let Some(progress) = parse_progress(&line) {
                let _ = app_clone.emit("ffmpeg://progress", progress);
            } else {
                let _ = app_clone.emit("ffmpeg://log", line);
            }
        }
    });

    tauri::async_runtime::spawn_blocking(move || loop {
        let mut guard = lock_running(&child_arc);
        match *guard {
            None => return -1,
            Some(ref mut p) => match p.child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(-1);
                    if code != 0 && p.cleanup_partial {
                        if let Some(path) = p.output_path.as_deref() {
                            cleanup_partial_output(path);
                        }
                    }
                    if cleanup_temp_on_exit {
                        cleanup_temp_files(&p.temp_files);
                    }
                    *guard = None;
                    return code;
                }
                Ok(None) => {
                    drop(guard);
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    if cleanup_temp_on_exit {
                        cleanup_temp_files(&p.temp_files);
                    }
                    *guard = None;
                    return -1;
                }
            },
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_ffmpeg(
    app: AppHandle,
    state: tauri::State<'_, FfmpegState>,
    operation: FfmpegOperation,
    cleanup_partial: Option<bool>,
) -> Result<(), String> {
    let cleanup_partial = cleanup_partial.unwrap_or(true);
    let running = Arc::clone(&state.running);

    let exit_code = match &operation {
        FfmpegOperation::Compress {
            input,
            output,
            preset,
            target_size_mb,
            ..
        } if target_size_mb.is_some_and(|v| v > 0.0) => {
            let duration = probe_media_inner(input.clone())
                .await
                .map(|m| m.duration_secs)
                .unwrap_or(0.0)
                .max(1.0);
            let TwoPassArgs {
                pass1_args,
                pass2_args,
                temp_files: pass_temp_files,
            } = build_compress_two_pass_args(
                input,
                output,
                *preset,
                target_size_mb.unwrap_or(1.0),
                duration,
            )
            .map_err(|e| e.to_string())?;

            let _ = app.emit("ffmpeg://log", "[hathor] two-pass size target: pass 1/2");
            let pass1_code = run_ffmpeg_once(
                &app,
                Arc::clone(&running),
                pass1_args,
                None,
                false,
                pass_temp_files.clone(),
                false,
            )
            .await?;
            if pass1_code != 0 {
                cleanup_temp_files(&pass_temp_files);
                pass1_code
            } else {
                let _ = app.emit("ffmpeg://log", "[hathor] two-pass size target: pass 2/2");
                run_ffmpeg_once(
                    &app,
                    Arc::clone(&running),
                    pass2_args,
                    Some(output.clone()),
                    cleanup_partial,
                    pass_temp_files,
                    true,
                )
                .await?
            }
        }
        _ => {
            let (args, temp_files) = build_args(&operation).map_err(|e| e.to_string())?;
            run_ffmpeg_once(
                &app,
                Arc::clone(&running),
                args,
                Some(operation.output_path().to_string()),
                cleanup_partial,
                temp_files,
                true,
            )
            .await?
        }
    };

    let _ = app.emit("ffmpeg://done", exit_code);
    Ok(())
}

#[tauri::command]
pub async fn cancel_ffmpeg(state: tauri::State<'_, FfmpegState>) -> Result<(), String> {
    let mut guard = lock_running(&state.running);
    if let Some(ref mut process) = *guard {
        process.child.kill().map_err(|e| e.to_string())?;
        if process.cleanup_partial {
            if let Some(path) = process.output_path.as_deref() {
                cleanup_partial_output(path);
            }
        }
        cleanup_temp_files(&process.temp_files);
    }
    Ok(())
}

#[tauri::command]
pub async fn probe_media(path: String) -> Result<MediaInfo, String> {
    probe_media_inner(path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn expand_media_inputs(paths: Vec<String>) -> Result<Vec<String>, String> {
    expand_media_inputs_inner(paths).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_output_path(path: String, policy: String) -> Result<String, String> {
    probe::resolve_output_path(path, &policy)
}
