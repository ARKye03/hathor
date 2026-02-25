use crate::ffmpeg::ops::{
    CompressPreset, FfmpegError, FfmpegOperation, ImageFormat, ImageResizeMethod,
    ImageSequenceFormat, LoudnessPreset, QualityMode,
};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn build_compress_single_pass_args(
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

pub struct TwoPassArgs {
    pub pass1_args: Vec<String>,
    pub pass2_args: Vec<String>,
    pub temp_files: Vec<String>,
}

pub fn build_compress_two_pass_args(
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
