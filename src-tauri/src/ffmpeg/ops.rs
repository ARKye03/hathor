use serde::Deserialize;

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

#[derive(Deserialize)]
pub struct CropSpec {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize)]
pub struct PadSpec {
    pub width: u32,
    pub height: u32,
    pub color: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FfmpegOperation {
    Convert {
        input: String,
        output: String,
        container: String,
        quality_mode: QualityMode,
        crf: Option<u32>,
        bitrate: Option<String>,
        resolution: Option<String>,
        fps: Option<u32>,
    },
    Trim {
        input: String,
        output: String,
        start: String,
        duration: String,
        trim_mode: Option<String>,
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
        flip: Option<String>,
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
        format: String,
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
        quality: u8,
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

impl FfmpegOperation {
    pub fn output_path(&self) -> &str {
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
