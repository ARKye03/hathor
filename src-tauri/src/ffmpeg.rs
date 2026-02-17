use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum FfmpegError {
    #[error("merge requires at least 2 input files")]
    MergeTooFewInputs,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("ffprobe failed: {0}")]
    ProbeFailed(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    SystemTime(#[from] std::time::SystemTimeError),
}

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

impl Default for FfmpegState {
    fn default() -> Self {
        Self::new()
    }
}

struct RunningProcess {
    child: Child,
    output_path: String,
    cleanup_partial: bool,
    temp_files: Vec<String>,
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
        trim_mode: Option<String>, // "fast" | "accurate"
    },
    Compress {
        input: String,
        output: String,
        crf: u32,
    },
    Transform {
        input: String,
        output: String,
        crop: Option<CropSpec>,
        pad: Option<PadSpec>,
        rotate: Option<u16>,
        flip: Option<String>, // "horizontal" | "vertical" | "both"
    },
    Merge {
        inputs: Vec<String>,
        output: String,
    },
    Remux {
        input: String,
        output: String,
    },
    ExtractAudio {
        input: String,
        output: String,
        format: String, // "mp3" | "aac" | "opus" | "wav"
    },
    ReplaceAudio {
        input: String,
        audio_input: String,
        output: String,
    },
    Loudness {
        input: String,
        output: String,
        preset: String, // "broadcast" | "streaming" | "podcast"
    },
    AudioControls {
        input: String,
        output: String,
        volume: f32,
        fade_in_secs: f32,
        fade_out_secs: f32,
    },
    ImageConvert {
        input: String,
        output: String,
        format: String, // "png" | "jpg" | "webp" | "avif" | "ico"
        quality: u8,    // 1..100
    },
    BurnSubtitles {
        input: String,
        subtitle_input: String,
        output: String,
    },
    ManageTracks {
        input: String,
        output: String,
        keep_audio_indices: Vec<u32>,
        keep_subtitle_indices: Vec<u32>,
        add_audio_input: Option<String>,
        add_subtitle_input: Option<String>,
    },
}

#[derive(Deserialize)]
pub struct CropSpec {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
pub struct PadSpec {
    width: u32,
    height: u32,
    color: String,
}

impl FfmpegOperation {
    fn output_path(&self) -> &str {
        match self {
            FfmpegOperation::Convert { output, .. } => output,
            FfmpegOperation::Trim { output, .. } => output,
            FfmpegOperation::Compress { output, .. } => output,
            FfmpegOperation::Transform { output, .. } => output,
            FfmpegOperation::Merge { output, .. } => output,
            FfmpegOperation::Remux { output, .. } => output,
            FfmpegOperation::ExtractAudio { output, .. } => output,
            FfmpegOperation::ReplaceAudio { output, .. } => output,
            FfmpegOperation::Loudness { output, .. } => output,
            FfmpegOperation::AudioControls { output, .. } => output,
            FfmpegOperation::ImageConvert { output, .. } => output,
            FfmpegOperation::BurnSubtitles { output, .. } => output,
            FfmpegOperation::ManageTracks { output, .. } => output,
        }
    }
}

fn create_concat_list(inputs: &[String]) -> Result<String, FfmpegError> {
    if inputs.len() < 2 {
        return Err(FfmpegError::MergeTooFewInputs);
    }
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path =
        std::env::temp_dir().join(format!("hathor-concat-{}-{}.txt", std::process::id(), ts));
    let mut body = String::new();
    for p in inputs {
        body.push_str("file '");
        body.push_str(&p.replace('\'', "'\\''"));
        body.push_str("'\n");
    }
    fs::write(&path, body)?;
    Ok(path.to_string_lossy().to_string())
}

fn output_ext(output: &str) -> String {
    output
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_else(|| "mp4".to_string())
}

fn escape_subtitles_filter_path(path: &str) -> String {
    path.replace('\\', "\\\\").replace(':', "\\:").replace('\'', "\\'")
}

pub fn build_args(op: &FfmpegOperation) -> Result<(Vec<String>, Vec<String>), FfmpegError> {
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

            if container == "gif" {
                let mut filters = Vec::<String>::new();
                if let Some(res) = resolution {
                    let h: Option<u32> = match res.as_str() {
                        "1080p" => Some(1080),
                        "720p" => Some(720),
                        "480p" => Some(480),
                        _ => None,
                    };
                    if let Some(h) = h {
                        filters.push(format!("scale=-2:{h}:flags=lanczos"));
                    }
                }
                if let Some(f) = fps {
                    filters.push(format!("fps={f}"));
                }
                let filter_base = if filters.is_empty() {
                    "fps=15".to_string()
                } else {
                    filters.join(",")
                };
                let palette_filter =
                    format!("{filter_base},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse");
                args.extend(["-vf".into(), palette_filter]);
                args.push(output.clone());
                return Ok((args, vec![]));
            }

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
            Ok((args, vec![]))
        }
        FfmpegOperation::Trim {
            input,
            output,
            start,
            duration,
            trim_mode,
        } => {
            if trim_mode.as_deref() == Some("fast") {
                Ok((
                    vec![
                        "-y".into(),
                        "-ss".into(),
                        start.clone(),
                        "-i".into(),
                        input.clone(),
                        "-t".into(),
                        duration.clone(),
                        "-c".into(),
                        "copy".into(),
                        "-avoid_negative_ts".into(),
                        "make_zero".into(),
                        output.clone(),
                    ],
                    vec![],
                ))
            } else {
                Ok((
                    vec![
                        "-y".into(),
                        "-i".into(),
                        input.clone(),
                        "-ss".into(),
                        start.clone(),
                        "-t".into(),
                        duration.clone(),
                        output.clone(),
                    ],
                    vec![],
                ))
            }
        }
        FfmpegOperation::Compress { input, output, crf } => Ok((
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-vcodec".into(),
                "libx264".into(),
                "-crf".into(),
                crf.to_string(),
                output.clone(),
            ],
            vec![],
        )),
        FfmpegOperation::Transform {
            input,
            output,
            crop,
            pad,
            rotate,
            flip,
        } => {
            let mut args = vec!["-y".into(), "-i".into(), input.clone()];
            let mut filters = Vec::<String>::new();

            if let Some(c) = crop {
                filters.push(format!("crop={}:{}:{}:{}", c.width, c.height, c.x, c.y));
            }

            if let Some(p) = pad {
                let color = if p.color.trim().is_empty() {
                    "black".to_string()
                } else {
                    p.color.clone()
                };
                filters.push(format!(
                    "pad={}:{}:(ow-iw)/2:(oh-ih)/2:{}",
                    p.width, p.height, color
                ));
            }

            if let Some(r) = rotate {
                match r {
                    90 => filters.push("transpose=1".into()),
                    180 => {
                        filters.push("hflip".into());
                        filters.push("vflip".into());
                    }
                    270 => filters.push("transpose=2".into()),
                    _ => {}
                }
            }

            if let Some(f) = flip {
                match f.as_str() {
                    "horizontal" => filters.push("hflip".into()),
                    "vertical" => filters.push("vflip".into()),
                    "both" => {
                        filters.push("hflip".into());
                        filters.push("vflip".into());
                    }
                    _ => {}
                }
            }

            if !filters.is_empty() {
                args.extend(["-vf".into(), filters.join(",")]);
            }

            args.extend([
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "medium".into(),
                "-crf".into(),
                "23".into(),
                "-pix_fmt".into(),
                "yuv420p".into(),
                "-c:a".into(),
                "copy".into(),
            ]);
            args.push(output.clone());
            Ok((args, vec![]))
        }
        FfmpegOperation::Merge { inputs, output } => {
            let list_path = create_concat_list(inputs)?;
            Ok((
                vec![
                    "-y".into(),
                    "-f".into(),
                    "concat".into(),
                    "-safe".into(),
                    "0".into(),
                    "-i".into(),
                    list_path.clone(),
                    "-c".into(),
                    "copy".into(),
                    output.clone(),
                ],
                vec![list_path],
            ))
        }
        FfmpegOperation::Remux { input, output } => Ok((
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-c".into(),
                "copy".into(),
                output.clone(),
            ],
            vec![],
        )),
        FfmpegOperation::ExtractAudio {
            input,
            output,
            format,
        } => {
            let mut args = vec!["-y".into(), "-i".into(), input.clone(), "-vn".into()];
            match format.as_str() {
                "aac" => args.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]),
                "opus" => args.extend([
                    "-c:a".into(),
                    "libopus".into(),
                    "-b:a".into(),
                    "128k".into(),
                ]),
                "wav" => args.extend(["-c:a".into(), "pcm_s16le".into()]),
                _ => args.extend(["-c:a".into(), "libmp3lame".into(), "-q:a".into(), "2".into()]),
            }
            args.push(output.clone());
            Ok((args, vec![]))
        }
        FfmpegOperation::ReplaceAudio {
            input,
            audio_input,
            output,
        } => Ok((
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-i".into(),
                audio_input.clone(),
                "-map".into(),
                "0:v?".into(),
                "-map".into(),
                "1:a:0".into(),
                "-map".into(),
                "0:s?".into(),
                "-c:v".into(),
                "copy".into(),
                "-c:s".into(),
                "copy".into(),
                "-c:a".into(),
                "aac".into(),
                "-shortest".into(),
                output.clone(),
            ],
            vec![],
        )),
        FfmpegOperation::Loudness {
            input,
            output,
            preset,
        } => {
            let (i, lra, tp) = match preset.as_str() {
                "streaming" => ("-16", "7", "-1.5"),
                "podcast" => ("-19", "8", "-2.0"),
                _ => ("-23", "7", "-2.0"),
            };
            Ok((
                vec![
                    "-y".into(),
                    "-i".into(),
                    input.clone(),
                    "-map".into(),
                    "0:v?".into(),
                    "-map".into(),
                    "0:a:0".into(),
                    "-map".into(),
                    "0:s?".into(),
                    "-c:v".into(),
                    "copy".into(),
                    "-c:s".into(),
                    "copy".into(),
                    "-af".into(),
                    format!("loudnorm=I={i}:LRA={lra}:TP={tp}"),
                    "-c:a".into(),
                    "aac".into(),
                    output.clone(),
                ],
                vec![],
            ))
        }
        FfmpegOperation::AudioControls {
            input,
            output,
            volume,
            fade_in_secs,
            fade_out_secs,
        } => {
            let mut filters = vec![format!("volume={:.3}", volume.max(0.0))];
            if *fade_in_secs > 0.0 {
                filters.push(format!(
                    "afade=t=in:st=0:d={:.3}",
                    fade_in_secs.max(0.0)
                ));
            }
            if *fade_out_secs > 0.0 {
                filters.push("areverse".into());
                filters.push(format!(
                    "afade=t=in:st=0:d={:.3}",
                    fade_out_secs.max(0.0)
                ));
                filters.push("areverse".into());
            }
            Ok((
                vec![
                    "-y".into(),
                    "-i".into(),
                    input.clone(),
                    "-map".into(),
                    "0:v?".into(),
                    "-map".into(),
                    "0:a:0".into(),
                    "-map".into(),
                    "0:s?".into(),
                    "-c:v".into(),
                    "copy".into(),
                    "-c:s".into(),
                    "copy".into(),
                    "-af".into(),
                    filters.join(","),
                    "-c:a".into(),
                    "aac".into(),
                    output.clone(),
                ],
                vec![],
            ))
        }
        FfmpegOperation::ImageConvert {
            input,
            output,
            format,
            quality,
        } => {
            let q = (*quality).clamp(1, 100);
            let mut args = vec!["-y".into(), "-i".into(), input.clone(), "-frames:v".into(), "1".into()];
            match format.as_str() {
                "jpg" => {
                    let jpg_q = ((100 - q as u32) * 30 / 99 + 2).to_string();
                    args.extend(["-c:v".into(), "mjpeg".into(), "-q:v".into(), jpg_q]);
                }
                "webp" => {
                    args.extend(["-c:v".into(), "libwebp".into(), "-q:v".into(), q.to_string()]);
                }
                "avif" => {
                    let crf = ((100 - q as u32) * 62 / 99).to_string();
                    args.extend([
                        "-c:v".into(),
                        "libaom-av1".into(),
                        "-still-picture".into(),
                        "1".into(),
                        "-crf".into(),
                        crf,
                        "-b:v".into(),
                        "0".into(),
                    ]);
                }
                "ico" => {
                    args.extend([
                        "-vf".into(),
                        "scale=256:256:force_original_aspect_ratio=decrease,pad=256:256:(ow-iw)/2:(oh-ih)/2:color=0x00000000".into(),
                        "-c:v".into(),
                        "png".into(),
                    ]);
                }
                _ => {
                    args.extend(["-c:v".into(), "png".into(), "-compression_level".into(), "6".into()]);
                }
            }
            args.push(output.clone());
            Ok((args, vec![]))
        }
        FfmpegOperation::BurnSubtitles {
            input,
            subtitle_input,
            output,
        } => Ok((
            vec![
                "-y".into(),
                "-i".into(),
                input.clone(),
                "-vf".into(),
                format!("subtitles='{}'", escape_subtitles_filter_path(subtitle_input)),
                "-c:v".into(),
                "libx264".into(),
                "-preset".into(),
                "medium".into(),
                "-crf".into(),
                "23".into(),
                "-c:a".into(),
                "copy".into(),
                output.clone(),
            ],
            vec![],
        )),
        FfmpegOperation::ManageTracks {
            input,
            output,
            keep_audio_indices,
            keep_subtitle_indices,
            add_audio_input,
            add_subtitle_input,
        } => {
            let mut args = vec!["-y".into(), "-i".into(), input.clone()];
            let mut next_input_idx = 1_u32;

            let add_audio_idx = if let Some(path) = add_audio_input.as_ref().filter(|p| !p.trim().is_empty()) {
                args.extend(["-i".into(), path.clone()]);
                let idx = next_input_idx;
                next_input_idx += 1;
                Some(idx)
            } else {
                None
            };

            let add_sub_idx = if let Some(path) = add_subtitle_input.as_ref().filter(|p| !p.trim().is_empty()) {
                args.extend(["-i".into(), path.clone()]);
                let idx = next_input_idx;
                Some(idx)
            } else {
                None
            };

            args.extend(["-map".into(), "0:v?".into()]);
            for idx in keep_audio_indices {
                args.extend(["-map".into(), format!("0:{idx}?")]);
            }
            for idx in keep_subtitle_indices {
                args.extend(["-map".into(), format!("0:{idx}?")]);
            }
            if let Some(idx) = add_audio_idx {
                args.extend(["-map".into(), format!("{idx}:a:0?")]);
            }
            if let Some(idx) = add_sub_idx {
                args.extend(["-map".into(), format!("{idx}:s:0?")]);
            }

            args.extend(["-c:v".into(), "copy".into()]);
            args.extend(["-c:a".into(), "copy".into()]);

            let has_subtitles = !keep_subtitle_indices.is_empty() || add_sub_idx.is_some();
            if has_subtitles {
                let ext = output_ext(output);
                if ext == "mp4" || ext == "mov" || ext == "m4v" {
                    args.extend(["-c:s".into(), "mov_text".into()]);
                } else {
                    args.extend(["-c:s".into(), "copy".into()]);
                }
            }

            args.push(output.clone());
            Ok((args, vec![]))
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

fn cleanup_temp_files(paths: &[String]) {
    for p in paths {
        if let Err(e) = fs::remove_file(p) {
            if e.kind() != std::io::ErrorKind::NotFound {
                eprintln!("failed to cleanup temp file '{}': {}", p, e);
            }
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
    let (args, temp_files) = build_args(&operation).map_err(|e| e.to_string())?;
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
        let mut guard = state.running.lock().unwrap_or_else(|p| p.into_inner());
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
        let mut guard = child_arc.lock().unwrap_or_else(|p| p.into_inner());
        match *guard {
            None => return -1, // cancelled — child was taken
            Some(ref mut p) => match p.child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().unwrap_or(-1);
                    if code != 0 && p.cleanup_partial {
                        cleanup_partial_output(&p.output_path);
                    }
                    cleanup_temp_files(&p.temp_files);
                    *guard = None;
                    return code;
                }
                Ok(None) => {
                    drop(guard);
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    cleanup_temp_files(&p.temp_files);
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
    let mut guard = state.running.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(ref mut process) = *guard {
        process.child.kill().map_err(|e| e.to_string())?;
        if process.cleanup_partial {
            cleanup_partial_output(&process.output_path);
        }
        cleanup_temp_files(&process.temp_files);
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

async fn probe_media_inner(path: String) -> Result<MediaInfo, FfmpegError> {
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
    })
    .await
    .map_err(|e| std::io::Error::other(e.to_string()))??;

    if !output.status.success() {
        return Err(FfmpegError::ProbeFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    let probe: ProbeOutput = serde_json::from_slice(&output.stdout)?;

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

#[tauri::command]
pub async fn probe_media(path: String) -> Result<MediaInfo, String> {
    probe_media_inner(path).await.map_err(|e| e.to_string())
}

const MEDIA_EXTENSIONS: [&str; 23] = [
    "mp4", "mkv", "mov", "webm", "m4v", "avi", "flv", "ts", "wmv", "mpg", "mpeg", "3gp", "mp3",
    "aac", "opus", "wav", "m4a", "png", "jpg", "jpeg", "webp", "avif", "ico",
];

fn is_media_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| MEDIA_EXTENSIONS.iter().any(|&e| ext.eq_ignore_ascii_case(e)))
        .unwrap_or(false)
}

fn collect_media_paths(path: &Path, out: &mut Vec<String>) -> Result<(), FfmpegError> {
    if path.is_file() {
        if is_media_file(path) {
            out.push(path.to_string_lossy().to_string());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let child = entry?.path();
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
        collect_media_paths(&path, &mut out).map_err(|e| e.to_string())?;
    }
    out.sort();
    out.dedup();
    Ok(out)
}
