use crate::ffmpeg::ops::FfmpegError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

pub async fn probe_media_inner(path: String) -> Result<MediaInfo, FfmpegError> {
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

pub fn expand_media_inputs(paths: Vec<String>) -> Result<Vec<String>, FfmpegError> {
    let mut out = Vec::<String>::new();
    for raw in paths {
        let path = PathBuf::from(raw);
        collect_media_paths(&path, &mut out)?;
    }
    out.sort();
    out.dedup();
    Ok(out)
}

pub fn resolve_output_path(path: String, policy: &str) -> Result<String, String> {
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
