use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

// ── Managed state ─────────────────────────────────────────────────────────────

pub struct FfmpegState {
    child: Arc<Mutex<Option<Child>>>,
}

impl FfmpegState {
    pub fn new() -> Self {
        Self { child: Arc::new(Mutex::new(None)) }
    }
}

// ── Operations ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FfmpegOperation {
    Convert { input: String, output: String },
    Trim { input: String, output: String, start: String, duration: String },
    Compress { input: String, output: String, crf: u32 },
}

pub fn build_args(op: &FfmpegOperation) -> Vec<String> {
    match op {
        FfmpegOperation::Convert { input, output } => {
            vec!["-y".into(), "-i".into(), input.clone(), output.clone()]
        }
        FfmpegOperation::Trim { input, output, start, duration } => {
            vec![
                "-y".into(), "-i".into(), input.clone(),
                "-ss".into(), start.clone(),
                "-t".into(), duration.clone(),
                output.clone(),
            ]
        }
        FfmpegOperation::Compress { input, output, crf } => {
            vec![
                "-y".into(), "-i".into(), input.clone(),
                "-vcodec".into(), "libx264".into(),
                "-crf".into(), crf.to_string(),
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
    let end = rest.find(|c: char| c.is_ascii_whitespace()).unwrap_or(rest.len());
    let val = rest.get(..end)?.trim();
    if val.is_empty() || val == "N/A" { None } else { Some(val.to_string()) }
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

// ── run_ffmpeg ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_ffmpeg(
    app: AppHandle,
    state: tauri::State<'_, FfmpegState>,
    operation: FfmpegOperation,
) -> Result<(), String> {
    let args = build_args(&operation);

    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stderr = child.stderr.take().unwrap();

    // Kill any previous job, store this child.
    {
        let mut guard = state.child.lock().unwrap();
        if let Some(ref mut prev) = *guard {
            let _ = prev.kill();
        }
        *guard = Some(child);
    }

    let child_arc = Arc::clone(&state.child);
    let app_clone = app.clone();

    // Read stderr in background.
    tauri::async_runtime::spawn_blocking(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
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
            Some(ref mut c) => match c.try_wait() {
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
    let mut guard = state.child.lock().unwrap();
    if let Some(ref mut child) = *guard {
        child.kill().map_err(|e| e.to_string())?;
        // Leave child in state; the wait loop will reap it via try_wait.
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
                "-v", "quiet",
                "-print_format", "json",
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

    let probe: ProbeOutput =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

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
        bit_rate: probe.format.bit_rate.as_deref().and_then(|b| b.parse().ok()),
        size_bytes: probe.format.size.as_deref().and_then(|s| s.parse().ok()),
        streams,
    })
}
