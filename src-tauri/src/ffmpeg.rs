use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
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
    output_path: Option<String>,
    cleanup_partial: bool,
    temp_files: Vec<String>,
}

fn lock_running<'a>(
    running: &'a Arc<Mutex<Option<RunningProcess>>>,
) -> MutexGuard<'a, Option<RunningProcess>> {
    running.lock().unwrap_or_else(|poisoned| {
        eprintln!("recovering from poisoned ffmpeg running-process mutex");
        poisoned.into_inner()
    })
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityMode {
    Crf,
    Bitrate,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompressPreset {
    IphoneIpad,
    Android,
    Youtube,
    Tiktok,
    Instagram,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoudnessPreset {
    Broadcast,
    Streaming,
    Podcast,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageSequenceFormat {
    Png,
    Jpg,
    Webp,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    Png,
    Jpg,
    Webp,
    Avif,
    Ico,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImageResizeMethod {
    Lanczos,
    Bicubic,
    Bilinear,
    Neighbor,
}

// ── Operations ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FfmpegOperation {
    Convert {
        input: String,
        output: String,
        container: String, // "mp4" | "mkv" | "mov" | "webm"
        quality_mode: QualityMode,
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
        preset: CompressPreset,
        target_size_mb: Option<f64>,
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
    Thumbnail {
        input: String,
        output: String,
        time: String,
    },
    ImageSequence {
        input: String,
        output_pattern: String,
        start: Option<String>,
        duration: Option<String>,
        fps: Option<u32>,
        scale_width: Option<u32>,
        format: ImageSequenceFormat,
    },
    GifMaker {
        input: String,
        output: String,
        start: Option<String>,
        duration: Option<String>,
        width: Option<u32>,
        fps: u32,
        use_palette: bool,
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
        preset: LoudnessPreset,
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
        format: ImageFormat,
        quality: u8, // 1..100
        resize_percent: Option<u16>,
        resize_method: Option<ImageResizeMethod>,
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
            FfmpegOperation::Thumbnail { output, .. } => output,
            FfmpegOperation::ImageSequence { output_pattern, .. } => output_pattern,
            FfmpegOperation::GifMaker { output, .. } => output,
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
    path.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

struct CompressPresetConfig {
    profile: &'static str,
    level: &'static str,
    audio_kbps: u32,
    faststart: bool,
}

fn compress_preset_config(preset: CompressPreset) -> CompressPresetConfig {
    match preset {
        CompressPreset::IphoneIpad => CompressPresetConfig {
            profile: "high",
            level: "4.1",
            audio_kbps: 160,
            faststart: true,
        },
        CompressPreset::Android => CompressPresetConfig {
            profile: "main",
            level: "4.0",
            audio_kbps: 128,
            faststart: true,
        },
        CompressPreset::Tiktok => CompressPresetConfig {
            profile: "high",
            level: "4.1",
            audio_kbps: 128,
            faststart: true,
        },
        CompressPreset::Instagram => CompressPresetConfig {
            profile: "high",
            level: "4.1",
            audio_kbps: 128,
            faststart: true,
        },
        CompressPreset::Youtube => CompressPresetConfig {
            profile: "high",
            level: "4.2",
            audio_kbps: 192,
            faststart: true,
        },
    }
}

fn apply_compress_preset_video_args(args: &mut Vec<String>, cfg: &CompressPresetConfig) {
    args.extend([
        "-pix_fmt".into(),
        "yuv420p".into(),
        "-profile:v".into(),
        cfg.profile.into(),
        "-level:v".into(),
        cfg.level.into(),
    ]);
}

fn maybe_faststart_arg(output: &str, cfg: &CompressPresetConfig, args: &mut Vec<String>) {
    if !cfg.faststart {
        return;
    }
    let ext = output_ext(output);
    if ext == "mp4" || ext == "mov" || ext == "m4v" {
        args.extend(["-movflags".into(), "+faststart".into()]);
    }
}

fn build_compress_single_pass_args(
    input: &str,
    output: &str,
    crf: u32,
    preset: CompressPreset,
) -> Vec<String> {
    let cfg = compress_preset_config(preset);
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        input.to_string(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-crf".into(),
        crf.to_string(),
    ];
    apply_compress_preset_video_args(&mut args, &cfg);
    args.extend([
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        format!("{}k", cfg.audio_kbps),
    ]);
    maybe_faststart_arg(output, &cfg, &mut args);
    args.push(output.to_string());
    args
}

fn create_passlog_prefix() -> Result<String, FfmpegError> {
    let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = std::env::temp_dir().join(format!("hathor-passlog-{}-{}", std::process::id(), ts));
    Ok(path.to_string_lossy().to_string())
}

struct TwoPassArgs {
    pass1_args: Vec<String>,
    pass2_args: Vec<String>,
    temp_files: Vec<String>,
}

fn build_compress_two_pass_args(
    input: &str,
    output: &str,
    preset: CompressPreset,
    target_size_mb: f64,
    duration_secs: f64,
) -> Result<TwoPassArgs, FfmpegError> {
    let cfg = compress_preset_config(preset);
    let passlog = create_passlog_prefix()?;
    let target_bytes = (target_size_mb.max(1.0) * 1024.0 * 1024.0) * 0.97;
    let total_kbps = (target_bytes * 8.0 / duration_secs.max(1.0) / 1000.0).max(250.0);
    let video_kbps = (total_kbps - cfg.audio_kbps as f64).max(200.0).round() as u64;
    let bitrate = format!("{video_kbps}k");
    let null_sink = if cfg!(target_os = "windows") {
        "NUL".to_string()
    } else {
        "/dev/null".to_string()
    };

    let mut pass1 = vec![
        "-y".into(),
        "-i".into(),
        input.to_string(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-b:v".into(),
        bitrate.clone(),
        "-pass".into(),
        "1".into(),
        "-passlogfile".into(),
        passlog.clone(),
        "-an".into(),
        "-f".into(),
        "mp4".into(),
    ];
    apply_compress_preset_video_args(&mut pass1, &cfg);
    pass1.push(null_sink);

    let mut pass2 = vec![
        "-y".into(),
        "-i".into(),
        input.to_string(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "medium".into(),
        "-b:v".into(),
        bitrate,
        "-pass".into(),
        "2".into(),
        "-passlogfile".into(),
        passlog.clone(),
    ];
    apply_compress_preset_video_args(&mut pass2, &cfg);
    pass2.extend([
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        format!("{}k", cfg.audio_kbps),
    ]);
    maybe_faststart_arg(output, &cfg, &mut pass2);
    pass2.push(output.to_string());

    let temp_files = vec![
        passlog.clone(),
        format!("{passlog}-0.log"),
        format!("{passlog}-0.log.mbtree"),
        format!("{passlog}.log"),
        format!("{passlog}.log.mbtree"),
    ];
    Ok(TwoPassArgs {
        pass1_args: pass1,
        pass2_args: pass2,
        temp_files,
    })
}

fn resolution_to_height(resolution: Option<&str>) -> Option<u32> {
    match resolution {
        Some("1080p") => Some(1080),
        Some("720p") => Some(720),
        Some("480p") => Some(480),
        _ => None,
    }
}

struct ConvertArgsInput<'a> {
    input: &'a str,
    output: &'a str,
    container: &'a str,
    quality_mode: QualityMode,
    crf: Option<u32>,
    bitrate: Option<&'a str>,
    resolution: Option<&'a str>,
    fps: Option<u32>,
}

fn build_convert_args(input: ConvertArgsInput<'_>) -> Vec<String> {
    let mut args = vec!["-y".into(), "-i".into(), input.input.to_string()];

    if input.container == "gif" {
        let mut filters = Vec::<String>::new();
        if let Some(h) = resolution_to_height(input.resolution) {
            filters.push(format!("scale=-2:{h}:flags=lanczos"));
        }
        if let Some(f) = input.fps {
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
        args.push(input.output.to_string());
        return args;
    }

    let webm = input.container == "webm";
    let vcodec = if webm { "libvpx-vp9" } else { "libx264" };
    let acodec = if webm { "libopus" } else { "aac" };
    args.extend(["-c:v".into(), vcodec.into()]);

    match input.quality_mode {
        QualityMode::Bitrate => {
            let bv = input.bitrate.unwrap_or("2000k");
            args.extend(["-b:v".into(), bv.into()]);
        }
        QualityMode::Crf => {
            args.extend(["-crf".into(), input.crf.unwrap_or(23).to_string()]);
            if webm {
                args.extend(["-b:v".into(), "0".into()]);
            }
        }
    }
    if !webm {
        args.extend(["-preset".into(), "medium".into()]);
    }

    let mut filters = Vec::<String>::new();
    if let Some(h) = resolution_to_height(input.resolution) {
        filters.push(format!("scale=-2:{h}"));
    }
    if let Some(f) = input.fps {
        filters.push(format!("fps={f}"));
    }
    if !filters.is_empty() {
        args.extend(["-vf".into(), filters.join(",")]);
    }

    args.extend(["-c:a".into(), acodec.into()]);
    args.push(input.output.to_string());
    args
}

fn build_image_sequence_args(
    input: &str,
    output_pattern: &str,
    start: Option<&str>,
    duration: Option<&str>,
    fps: Option<u32>,
    scale_width: Option<u32>,
    format: ImageSequenceFormat,
) -> Vec<String> {
    let mut args = vec!["-y".into(), "-i".into(), input.to_string()];
    if let Some(s) = start.filter(|s| !s.trim().is_empty()) {
        args.extend(["-ss".into(), s.to_string()]);
    }
    if let Some(d) = duration.filter(|d| !d.trim().is_empty()) {
        args.extend(["-t".into(), d.to_string()]);
    }

    let mut filters = Vec::<String>::new();
    if let Some(v) = fps {
        filters.push(format!("fps={}", v.max(1)));
    }
    if let Some(w) = scale_width.filter(|w| *w > 0) {
        filters.push(format!("scale={w}:-1"));
    }
    if !filters.is_empty() {
        args.extend(["-vf".into(), filters.join(",")]);
    }

    match format {
        ImageSequenceFormat::Jpg => {
            args.extend(["-c:v".into(), "mjpeg".into(), "-q:v".into(), "2".into()])
        }
        ImageSequenceFormat::Webp => {
            args.extend(["-c:v".into(), "libwebp".into(), "-q:v".into(), "80".into()])
        }
        ImageSequenceFormat::Png => args.extend(["-c:v".into(), "png".into()]),
    }
    args.push(output_pattern.to_string());
    args
}

fn build_image_convert_args(
    input: &str,
    output: &str,
    format: ImageFormat,
    quality: u8,
    resize_percent: Option<u16>,
    resize_method: Option<ImageResizeMethod>,
) -> Vec<String> {
    let q = quality.clamp(1, 100);
    let mut args = vec![
        "-y".into(),
        "-i".into(),
        input.to_string(),
        "-frames:v".into(),
        "1".into(),
    ];
    if format != ImageFormat::Ico {
        if let Some(p) = resize_percent
            .map(|pct| pct.clamp(1, 400))
            .filter(|pct| *pct != 100)
        {
            let method = match resize_method.unwrap_or(ImageResizeMethod::Lanczos) {
                ImageResizeMethod::Bicubic => "bicubic",
                ImageResizeMethod::Bilinear => "bilinear",
                ImageResizeMethod::Neighbor => "neighbor",
                ImageResizeMethod::Lanczos => "lanczos",
            };
            args.extend([
                "-vf".into(),
                format!("scale=iw*{p}/100:ih*{p}/100:flags={method}"),
            ]);
        }
    }
    match format {
        ImageFormat::Jpg => {
            let jpg_q = ((100 - q as u32) * 30 / 99 + 2).to_string();
            args.extend(["-c:v".into(), "mjpeg".into(), "-q:v".into(), jpg_q]);
        }
        ImageFormat::Webp => {
            args.extend([
                "-c:v".into(),
                "libwebp".into(),
                "-q:v".into(),
                q.to_string(),
            ]);
        }
        ImageFormat::Avif => {
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
        ImageFormat::Ico => {
            args.extend([
                "-vf".into(),
                "scale=256:256:force_original_aspect_ratio=decrease,pad=256:256:(ow-iw)/2:(oh-ih)/2:color=0x00000000".into(),
                "-c:v".into(),
                "png".into(),
            ]);
        }
        ImageFormat::Png => {
            args.extend([
                "-c:v".into(),
                "png".into(),
                "-compression_level".into(),
                "6".into(),
            ]);
        }
    }
    args.push(output.to_string());
    args
}

fn build_manage_tracks_args(
    input: &str,
    output: &str,
    keep_audio_indices: &[u32],
    keep_subtitle_indices: &[u32],
    add_audio_input: Option<&str>,
    add_subtitle_input: Option<&str>,
) -> Vec<String> {
    let mut args = vec!["-y".into(), "-i".into(), input.to_string()];
    let mut next_input_idx = 1_u32;

    let add_audio_idx = if let Some(path) = add_audio_input.filter(|p| !p.trim().is_empty()) {
        args.extend(["-i".into(), path.to_string()]);
        let idx = next_input_idx;
        next_input_idx += 1;
        Some(idx)
    } else {
        None
    };

    let add_sub_idx = if let Some(path) = add_subtitle_input.filter(|p| !p.trim().is_empty()) {
        args.extend(["-i".into(), path.to_string()]);
        Some(next_input_idx)
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

    args.push(output.to_string());
    args
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
        } => Ok((
            build_convert_args(ConvertArgsInput {
                input,
                output,
                container,
                quality_mode: *quality_mode,
                crf: *crf,
                bitrate: bitrate.as_deref(),
                resolution: resolution.as_deref(),
                fps: *fps,
            }),
            vec![],
        )),
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
        FfmpegOperation::Compress {
            input,
            output,
            crf,
            preset,
            ..
        } => Ok((
            build_compress_single_pass_args(input, output, *crf, *preset),
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
        FfmpegOperation::Thumbnail {
            input,
            output,
            time,
        } => Ok((
            vec![
                "-y".into(),
                "-ss".into(),
                time.clone(),
                "-i".into(),
                input.clone(),
                "-frames:v".into(),
                "1".into(),
                output.clone(),
            ],
            vec![],
        )),
        FfmpegOperation::ImageSequence {
            input,
            output_pattern,
            start,
            duration,
            fps,
            scale_width,
            format,
        } => Ok((
            build_image_sequence_args(
                input,
                output_pattern,
                start.as_deref(),
                duration.as_deref(),
                *fps,
                *scale_width,
                *format,
            ),
            vec![],
        )),
        FfmpegOperation::GifMaker {
            input,
            output,
            start,
            duration,
            width,
            fps,
            use_palette,
        } => {
            let mut args = vec!["-y".into()];
            if let Some(s) = start.as_ref().filter(|s| !s.trim().is_empty()) {
                args.extend(["-ss".into(), s.clone()]);
            }
            args.extend(["-i".into(), input.clone()]);
            if let Some(d) = duration.as_ref().filter(|d| !d.trim().is_empty()) {
                args.extend(["-t".into(), d.clone()]);
            }

            let fps = (*fps).max(1);
            let width = width.unwrap_or(480).max(80);
            let base = format!("fps={fps},scale={width}:-1:flags=lanczos");
            if *use_palette {
                args.extend([
                    "-vf".into(),
                    format!("{base},split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"),
                ]);
            } else {
                args.extend(["-vf".into(), base]);
            }
            args.push(output.clone());
            Ok((args, vec![]))
        }
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
                _ => args.extend([
                    "-c:a".into(),
                    "libmp3lame".into(),
                    "-q:a".into(),
                    "2".into(),
                ]),
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
            let (i, lra, tp) = match *preset {
                LoudnessPreset::Streaming => ("-16", "7", "-1.5"),
                LoudnessPreset::Podcast => ("-19", "8", "-2.0"),
                LoudnessPreset::Broadcast => ("-23", "7", "-2.0"),
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
                filters.push(format!("afade=t=in:st=0:d={:.3}", fade_in_secs.max(0.0)));
            }
            if *fade_out_secs > 0.0 {
                filters.push("areverse".into());
                filters.push(format!("afade=t=in:st=0:d={:.3}", fade_out_secs.max(0.0)));
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
            resize_percent,
            resize_method,
        } => Ok((
            build_image_convert_args(
                input,
                output,
                *format,
                *quality,
                *resize_percent,
                *resize_method,
            ),
            vec![],
        )),
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
                format!(
                    "subtitles='{}'",
                    escape_subtitles_filter_path(subtitle_input)
                ),
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
        } => Ok((
            build_manage_tracks_args(
                input,
                output,
                keep_audio_indices,
                keep_subtitle_indices,
                add_audio_input.as_deref(),
                add_subtitle_input.as_deref(),
            ),
            vec![],
        )),
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
        .map(|ext| {
            MEDIA_EXTENSIONS
                .iter()
                .any(|&e| ext.eq_ignore_ascii_case(e))
        })
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

#[tauri::command]
pub async fn resolve_output_path(path: String, policy: String) -> Result<String, String> {
    if policy != "auto_increment" {
        return Ok(path);
    }

    let base = PathBuf::from(&path);
    if !base.exists() {
        return Ok(path);
    }

    let parent = base.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output")
        .to_string();
    let ext = base
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_string());

    for i in 1..=9999_u32 {
        let candidate_name = match &ext {
            Some(ext) if !ext.is_empty() => format!("{stem}_{i:03}.{ext}"),
            _ => format!("{stem}_{i:03}"),
        };
        let candidate = if parent.as_os_str().is_empty() {
            PathBuf::from(candidate_name)
        } else {
            parent.join(candidate_name)
        };
        if !candidate.exists() {
            return Ok(candidate.to_string_lossy().to_string());
        }
    }

    Err("failed to resolve unique output filename".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_uses_bitrate_when_quality_mode_is_bitrate() {
        let op = FfmpegOperation::Convert {
            input: "in.mp4".into(),
            output: "out.mp4".into(),
            container: "mp4".into(),
            quality_mode: QualityMode::Bitrate,
            crf: Some(18),
            bitrate: Some("1800k".into()),
            resolution: None,
            fps: None,
        };
        let (args, _) = build_args(&op).expect("convert args should build");
        assert!(args.windows(2).any(|w| w[0] == "-b:v" && w[1] == "1800k"));
        assert!(!args.iter().any(|a| a == "-crf"));
    }

    #[test]
    fn image_convert_applies_resize_filter_for_non_ico() {
        let op = FfmpegOperation::ImageConvert {
            input: "in.png".into(),
            output: "out.webp".into(),
            format: ImageFormat::Webp,
            quality: 80,
            resize_percent: Some(50),
            resize_method: Some(ImageResizeMethod::Lanczos),
        };
        let (args, _) = build_args(&op).expect("image convert args should build");
        assert!(args
            .iter()
            .any(|a| a.contains("scale=iw*50/100:ih*50/100:flags=lanczos")));
    }

    #[test]
    fn image_convert_ico_ignores_resize_and_forces_icon_filter() {
        let op = FfmpegOperation::ImageConvert {
            input: "in.png".into(),
            output: "out.ico".into(),
            format: ImageFormat::Ico,
            quality: 80,
            resize_percent: Some(50),
            resize_method: Some(ImageResizeMethod::Bicubic),
        };
        let (args, _) = build_args(&op).expect("ico convert args should build");
        assert!(args.iter().any(|a| a.contains("scale=256:256")));
        assert!(!args.iter().any(|a| a.contains("scale=iw*50/100:ih*50/100")));
    }

    #[test]
    fn manage_tracks_adds_expected_maps() {
        let op = FfmpegOperation::ManageTracks {
            input: "in.mkv".into(),
            output: "out.mkv".into(),
            keep_audio_indices: vec![1, 2],
            keep_subtitle_indices: vec![3],
            add_audio_input: Some("new_audio.aac".into()),
            add_subtitle_input: Some("new_subs.srt".into()),
        };
        let (args, _) = build_args(&op).expect("manage tracks args should build");
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "0:1?"));
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "0:2?"));
        assert!(args.windows(2).any(|w| w[0] == "-map" && w[1] == "0:3?"));
        assert!(args.iter().any(|a| a == "1:a:0?" || a == "2:a:0?"));
    }
}
