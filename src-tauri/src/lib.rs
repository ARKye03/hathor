mod ffmpeg;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ffmpeg::FfmpegState::new())
        .invoke_handler(tauri::generate_handler![
            ffmpeg::run_ffmpeg,
            ffmpeg::cancel_ffmpeg,
            ffmpeg::probe_media,
            ffmpeg::expand_media_inputs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
