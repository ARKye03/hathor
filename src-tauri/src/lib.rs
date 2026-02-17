mod ffmpeg;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ffmpeg::FfmpegState::new())
        .invoke_handler(tauri::generate_handler![greet, ffmpeg::run_ffmpeg, ffmpeg::cancel_ffmpeg, ffmpeg::probe_media])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
