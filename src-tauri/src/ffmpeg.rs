use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ── Managed state ─────────────────────────────────────────────────────────────

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

struct RunningProcess {
    child: Child,
    output_path: String,
    cleanup_partial: bool,
}

// ── Operations ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FfmpegOperation {
    Convert {
        input: String,
        output: String,
        container: String,    // "mp4" | "mkv" | "mov" | "webm"
        quality_mode: String, // "crf" | "bitrate"
        crf: Option<u32>,
        bitrate: Option<String>,    // e.g. "2000k"
        resolution: Option<String>, // "1080p" | "720p" | "480p" | null = keep
        fps: Option<u32>,           // null = keep
    },
    Trim {
        input: String,
        output: String,
        start: String,
        duration: String,
    },
    Compress {
        input: String,
        output: String,
        crf: u32,
    },
    Remux {
        input: String,
        output: String,
    },
}

impl FfmpegOperation {
    fn output_path(&self) -> &str {
        match self {
            FfmpegOperation::Convert { output, .. } => output,
            FfmpegOperation::Trim { output, .. } => output,
            FfmpegOperation::Compress { output, .. } => output,
            FfmpegOperation::Remux { output, .. } => output,
        }
    }
}

pub fn build_args(op: &FfmpegOperation) -> Vec<String> {
    match op {
        FfmpegOperation::Convert {
            input,
            output,
            container,
            quality_mode,
            crf,
            bitrate,
            resolution,
            fps,
        } => {
            let mut args = vec!["-y".into(), "-i".into(), input.clone()];

            let webm = container == "webm";
            let vcodec = if webm { "libvpx-vp9" } else { "libx264" };
            let acodec = if webm { "libopus" } else { "aac" };

            args.extend(["-c:v".into(), vcodec.into()]);

            match quality_mode.as_str() {
                "bitrate" => {
                    let bv = bitrate.as_deref().unwrap_or("2000k");
                    args.extend(["-b:v".into(), bv.into()]);
                }
                _ => {
                    let q = crf.unwrap_or(23);
                    args.extend(["-crf".into(), q.to_string()]);
                    if webm {
                        args.extend(["-b:v".into(), "0".into()]);
                    }
                }
            }
            if !webm {
                args.extend(["-preset".into(), "medium".into()]);
            }

            // Video filters: scale + fps
            let mut filters = Vec::<String>::new();
            if let Some(res) = resolution {
                let h: Option<u32> = match res.as_str() {
                    "1080p" => Some(1080),
                    "720p" => Some(720),
                    "480p" => Some(480),
                    _ => None,
                };
                if let Some(h) = h {
                    filters.push(format!("scale=-2:{h}"));
                }
            }
            if let Some(f) = fps {
                filters.push(format!("fps={f}"));
            }
            if !filters.is_empty() {
                args.extend(["-vf".into(), filters.join(",")]);
            }

            args.extend(["-c:a".into(), acodec.into()]);
            args.push(output.clone());
            args
        }
        FfmpegOperation::Trim {
            input,
            output,
            start,
            duration,
        } => {
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-ss".into(),
                start.clone(),
                "-t".into(),
                duration.clone(),
                output.clone(),
            ]
        }
        FfmpegOperation::Compress { input, output, crf } => {
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-vcodec".into(),
                "libx264".into(),
                "-crf".into(),
                crf.to_string(),
                output.clone(),
            ]
        }
        FfmpegOperation::Remux { input, output } => {
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-c".into(),
                "copy".into(),
                output.clone(),
            ]
        }
    }
}

// ── Progress ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct FfmpegProgress {
    pub frame: Option<u64>,
    pub fps: Option<f64>,
    pub time: Option<String>,
    pub time_secs: Option<f64>,
    pub speed: Option<f64>,
    pub bitrate: Option<String>,
    pub size_kb: Option<u64>,
}

fn extract_field(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)?.checked_add(key.len())?;
    let rest = line.get(start..)?.trim_start();
    let end = rest
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(rest.len());
    let val = rest.get(..end)?.trim();
    if val.is_empty() || val == "N/A" {
        None
    } else {
        Some(val.to_string())
    }
}

fn time_to_secs(t: &str) -> Option<f64> {
    let mut parts = t.splitn(3, ':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

fn parse_progress(line: &str) -> Option<FfmpegProgress> {
    // FFmpeg progress lines always contain "time=" and either "frame=" or "speed="
    if !line.contains("time=") || (!line.contains("frame=") && !line.contains("speed=")) {
        return None;
    }
    let time_str = extract_field(line, "time=");
    // Reject lines where time is 0:00:00 placeholder before encode starts
    let time_secs = time_str.as_deref().and_then(time_to_secs);

    Some(FfmpegProgress {
        frame: extract_field(line, "frame=").and_then(|v| v.parse().ok()),
        fps: extract_field(line, "fps=").and_then(|v| v.parse().ok()),
        time: time_str,
        time_secs,
        speed: extract_field(line, "speed=")
            .map(|v| v.trim_end_matches('x').to_string())
            .and_then(|v| v.parse().ok()),
        bitrate: extract_field(line, "bitrate="),
        // handles both "size=" (ongoing) and "Lsize=" (final line)
        size_kb: extract_field(line, "size=")
            .map(|v| v.trim_end_matches("kB").trim().to_string())
            .and_then(|v| v.parse().ok()),
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

// ── run_ffmpeg ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_ffmpeg(
    app: AppHandle,
    state: tauri::State<'_, FfmpegState>,
    operation: FfmpegOperation,
    cleanup_partial: Option<bool>,
) -> Result<(), String> {
    let args = build_args(&operation);
    let output_path = operation.output_path().to_string();
    let cleanup_partial = cleanup_partial.unwrap_or(true);
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

    // Kill any previous job, store this child.
    {
        let mut guard = state.running.lock().unwrap();
        if let Some(ref mut prev) = *guard {
            let _ = prev.child.kill();
        }
        *guard = Some(RunningProcess {
            child,
            output_path,
            cleanup_partial,
        });
    }

    let child_arc = Arc::clone(&state.running);
    let app_clone = app.clone();

    // Read stderr in background, splitting on \r and \n.
    // FFmpeg writes progress lines with \r (not \n), so BufReader::lines() would
    // buffer everything until EOF. We read in chunks and split manually.
    tauri::async_runtime::spawn_blocking(move || {
        let mut reader = BufReader::new(stderr);
        let mut pending = String::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.push_str(&String::from_utf8_lossy(&buf[..n]));
                    while let Some(pos) = pending.find(|c| c == '\r' || c == '\n') {
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
        // Flush anything left without a terminator.
        let line = pending.trim().to_string();
        if !line.is_empty() {
            if let Some(progress) = parse_progress(&line) {
                let _ = app_clone.emit("ffmpeg://progress", progress);
            } else {
                let _ = app_clone.emit("ffmpeg://log", line);
            }
        }
    });

    // Poll try_wait so cancel_ffmpeg can call kill() without deadlocking.
    let exit_code = tauri::async_runtime::spawn_blocking(move || loop {
        let mut guard = child_arc.lock().unwrap();
        match *guard {
            None => return -1, // cancelled — child was taken
            Some(ref mut p) => match p.child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(-1);
                    *guard = None;
                    return code;
                }
                Ok(None) => {
                    drop(guard);
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    *guard = None;
                    return -1;
                }
            },
        }
    })
    .await
    .map_err(|e| e.to_string())?;

    let _ = app.emit("ffmpeg://done", exit_code);
    Ok(())
}

#[tauri::command]
pub async fn cancel_ffmpeg(state: tauri::State<'_, FfmpegState>) -> Result<(), String> {
    let mut guard = state.running.lock().unwrap();
    if let Some(ref mut process) = *guard {
        process.child.kill().map_err(|e| e.to_string())?;
        if process.cleanup_partial {
            cleanup_partial_output(&process.output_path);
        }
        // Leave process in state; the wait loop will reap it via try_wait.
    }
    Ok(())
}

// ── probe_media ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct MediaInfo {
    pub duration_secs: f64,
    pub format: String,
    pub bit_rate: Option<u64>,
    pub size_bytes: Option<u64>,
    pub streams: Vec<StreamInfo>,
}

#[derive(Serialize)]
pub struct StreamInfo {
    pub index: u32,
    pub codec_type: String,
    pub codec_name: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<String>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub language: Option<String>,
}

// Raw ffprobe JSON shapes
#[derive(Deserialize)]
struct ProbeOutput {
    format: ProbeFormat,
    streams: Vec<ProbeStream>,
}

#[derive(Deserialize)]
struct ProbeFormat {
    format_name: String,
    duration: Option<String>,
    bit_rate: Option<String>,
    size: Option<String>,
}

#[derive(Deserialize)]
struct ProbeStream {
    index: u32,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    tags: Option<ProbeTags>,
}

#[derive(Deserialize)]
struct ProbeTags {
    language: Option<String>,
    #[serde(rename = "LANGUAGE")]
    language_upper: Option<String>,
}

#[tauri::command]
pub async fn probe_media(path: String) -> Result<MediaInfo, String> {
    let output = tauri::async_runtime::spawn_blocking(move || {
        Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                &path,
            ])
            .output()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }

    let probe: ProbeOutput = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

    let streams = probe
        .streams
        .into_iter()
        .map(|s| StreamInfo {
            index: s.index,
            codec_type: s.codec_type.unwrap_or_default(),
            codec_name: s.codec_name.unwrap_or_default(),
            width: s.width,
            height: s.height,
            frame_rate: s.r_frame_rate,
            sample_rate: s.sample_rate.and_then(|r| r.parse().ok()),
            channels: s.channels,
            language: s.tags.and_then(|t| t.language.or(t.language_upper)),
        })
        .collect();

    Ok(MediaInfo {
        duration_secs: probe
            .format
            .duration
            .as_deref()
            .and_then(|d| d.parse().ok())
            .unwrap_or(0.0),
        format: probe.format.format_name,
        bit_rate: probe
            .format
            .bit_rate
            .as_deref()
            .and_then(|b| b.parse().ok()),
        size_bytes: probe.format.size.as_deref().and_then(|s| s.parse().ok()),
        streams,
    })
}

const MEDIA_EXTENSIONS: [&str; 17] = [
    "mp4", "mkv", "mov", "webm", "m4v", "avi", "flv", "ts", "wmv", "mpg", "mpeg", "3gp", "mp3",
    "aac", "opus", "wav", "m4a",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MEDIA_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_media_paths(path: &Path, out: &mut Vec<String>) -> Result<(), String> {
    if path.is_file() {
        if is_media_file(path) {
            out.push(path.to_string_lossy().to_string());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let child = entry.path();
        if child.is_dir() {
            collect_media_paths(&child, out)?;
        } else if child.is_file() && is_media_file(&child) {
            out.push(child.to_string_lossy().to_string());
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn expand_media_inputs(paths: Vec<String>) -> Result<Vec<String>, String> {
    let mut out = Vec::<String>::new();
    for raw in paths {
        let path = PathBuf::from(raw);
        collect_media_paths(&path, &mut out)?;
    }
    out.sort();
    out.dedup();
    Ok(out)
}
