use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FfmpegOperation {
    Convert {
        input: String,
        output: String,
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
}

pub fn build_args(op: &FfmpegOperation) -> Vec<String> {
    match op {
        FfmpegOperation::Convert { input, output } => {
            vec!["-y".into(), "-i".into(), input.clone(), output.clone()]
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
    }
}

#[tauri::command]
pub async fn run_ffmpeg(app: AppHandle, operation: FfmpegOperation) -> Result<(), String> {
    let args = build_args(&operation);

    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stderr = child.stderr.take().unwrap();
    let app_clone = app.clone();

    // Stream stderr line-by-line as log events (ffmpeg writes progress to stderr)
    tauri::async_runtime::spawn_blocking(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            let _ = app_clone.emit("ffmpeg://log", line);
        }
    });

    let status =
        tauri::async_runtime::spawn_blocking(move || child.wait().map_err(|e| e.to_string()))
            .await
            .map_err(|e| e.to_string())??;

    let _ = app.emit("ffmpeg://done", status.code().unwrap_or(-1));

    Ok(())
}
