// P3 字幕後端模組。
mod subs;
mod settings;
mod sub_memory;
mod playback_memory;
mod recent;
mod fsutil;
mod capture;

use tauri::Manager as _;

/// Returns the canonical Cinelingo data root: %LOCALAPPDATA%\Cinelingo-data.
/// All app data (settings, models, engine, cache) lives under this single directory.
pub fn data_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().local_data_dir().map_err(|e| e.to_string())?.join("Cinelingo-data"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_libmpv::init())
        .plugin(tauri_plugin_opener::init())
        .manage(subs::SubsState::default())
        .setup(|app| {
            // 讓 mpv 內建 ytdl_hook 經 PATH 找到 app 打包的 yt-dlp（含日後才下載的）。
            // 修「app 自解的 googlevideo URL 被 YouTube 限速、cold seek 卡死」：改由 mpv ytdl
            // 重新解析 URL（不限速、可 seek），而 ytdl_hook 是在 loadfile 時經 PATH exec yt-dlp。
            if let Ok(subs_dir) = data_dir(app.handle()).map(|d| d.join("subs")) {
                let prev = std::env::var_os("PATH").unwrap_or_default();
                let mut paths = vec![subs_dir];
                paths.extend(std::env::split_paths(&prev));
                if let Ok(joined) = std::env::join_paths(paths) {
                    std::env::set_var("PATH", joined);
                }
            }
            // 讓 ort（load-dynamic）找到自管釘版 onnxruntime.dll。
            // dev：copy-libmpv 已把 DLL 放 current_exe() 同層；bundle：resources["lib/**/*"] 解到 resource_dir/lib。
            // 禁寫死 src-tauri/lib（build-time 路徑、runtime 不存在）。
            if std::env::var_os("ORT_DYLIB_PATH").is_none() {
                let mut candidates: Vec<std::path::PathBuf> = Vec::new();
                if let Ok(exe) = std::env::current_exe() {
                    if let Some(dir) = exe.parent() {
                        candidates.push(dir.join("onnxruntime.dll"));
                    }
                }
                if let Ok(res) = app.path().resource_dir() {
                    candidates.push(res.join("lib").join("onnxruntime.dll"));
                    candidates.push(res.join("onnxruntime.dll"));
                }
                if let Some(dll) = candidates.into_iter().find(|p| p.exists()) {
                    std::env::set_var("ORT_DYLIB_PATH", dll);
                } else {
                    eprintln!("[vad] 找不到 onnxruntime.dll（候選皆不存在）；loopback VAD 將無法啟動");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            subs::start_transcription,
            subs::stop_transcription,
            subs::notify_seek,
            subs::detect_hardware,
            subs::list_models,
            subs::download_model,
            subs::check_engine,
            subs::provision_engine,
            subs::read_text_file,
            subs::sidecar::list_sidecar_subs,
            subs::check_ytdlp,
            subs::provision_ytdlp,
            subs::resolve_remote,
            subs::enumerate_playlist,
            subs::remote_title,
            subs::list_audio_sources,
            subs::arm_audio_source,
            subs::disarm_audio_source,
            subs::start_external_transcription,
            subs::stop_external_transcription,
            subs::translate_engine_ready,
            subs::check_translate_engine,
            subs::provision_translate_engine,
            settings::load_settings,
            settings::save_settings,
            sub_memory::load_sub_memory,
            sub_memory::save_sub_memory,
            playback_memory::load_playback_memory,
            playback_memory::save_playback_memory,
            recent::load_recent,
            recent::save_recent,
            fsutil::expand_playable_paths,
            fsutil::path_exists
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                use tauri::Manager;
                let inner = handle.state::<subs::SubsState>().inner.clone();
                tauri::async_runtime::block_on(async move {
                    inner.lock().await.shutdown().await;
                });
            }
        });
}
