pub mod cue;
pub mod plan;
pub mod merge;
pub mod hwdetect;
pub mod download;
pub mod audio;
pub mod whisper;
pub mod session;
pub mod sidecar;
pub mod cache;
pub mod remote;
pub mod vad;
pub mod hallucination;
pub mod stream;

use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct SubsState {
    pub inner: Arc<Mutex<session::Manager>>,
    pub downloading: Arc<std::sync::Mutex<HashSet<String>>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResetEvent {
    pub session_id: String,
    pub no_clock: bool, // true = loopback（前端走 no-clock render）；false = 檔案/remote 播放
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub phase: String,
    pub done: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadEvent {
    pub key: String,
    pub phase: String,            // "downloading" | "done" | "error"
    pub done: u64,
    pub total: Option<u64>,       // 序列化為 null（前端型別 number | null）
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    pub key: String,
    pub downloaded: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingAsset {
    pub kind: String, // "model" | "vad" | "backend" | "ffmpeg"
    pub size_mb: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub backend_kind: String, // "cuda" | "vulkan" | "cpu"（顯示用）
    pub missing: Vec<MissingAsset>, // 空 = 已備妥
}

#[tauri::command]
pub async fn detect_hardware() -> Result<hwdetect::HwInfo, String> {
    tauri::async_runtime::spawn_blocking(hwdetect::detect_hardware_blocking)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_models(app: tauri::AppHandle) -> Result<Vec<ModelStatus>, String> {
    let subs_dir = crate::data_dir(&app)?.join("subs");
    let keys = ["small", "medium", "turbo", "large-v3"];
    Ok(keys
        .iter()
        .map(|k| ModelStatus { key: k.to_string(), downloaded: download::model_downloaded_in(&subs_dir, k) })
        .collect())
}

#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    key: String,
) -> Result<(), String> {
    let subs_dir = crate::data_dir(&app)?.join("subs");
    download::ensure_model_file(&app, &subs_dir, &state.downloading, &key, true, |_, _| {})
        .await
        .map(|_| ())
}

/// 唯讀：回報引擎缺哪些件 + 各約略大小（給「詢問」對話框）。
#[tauri::command]
pub async fn check_engine(app: tauri::AppHandle) -> Result<EngineStatus, String> {
    let subs_dir = crate::data_dir(&app)?.join("subs");
    let hw = tauri::async_runtime::spawn_blocking(hwdetect::detect_hardware_blocking)
        .await
        .map_err(|e| e.to_string())?;
    let mut missing = Vec::new();
    if !download::any_model_downloaded(&subs_dir) {
        missing.push(MissingAsset { kind: "model".into(), size_mb: download::SIZE_MB_TURBO });
    }
    if !subs_dir.join(download::MODEL_VAD.filename).exists() {
        missing.push(MissingAsset { kind: "vad".into(), size_mb: download::SIZE_MB_VAD });
    }
    if download::find_exe(&subs_dir.join("whisper"), "whisper-server.exe").is_none() {
        missing.push(MissingAsset { kind: "backend".into(), size_mb: download::asset_size_mb("backend", &hw.backend) });
    }
    if download::find_exe(&subs_dir.join("ffmpeg"), "ffmpeg.exe").is_none() {
        missing.push(MissingAsset { kind: "ffmpeg".into(), size_mb: download::SIZE_MB_FFMPEG });
    }
    Ok(EngineStatus { backend_kind: hw.backend, missing })
}

/// 下載所有缺的引擎件（不可取消）；無任一模型則抓 turbo。模型走 emit_progress=false（abort 不變式）。
#[tauri::command]
pub async fn provision_engine(app: tauri::AppHandle, state: tauri::State<'_, SubsState>) -> Result<(), String> {
    use std::sync::atomic::AtomicBool;
    let data = crate::data_dir(&app)?.join("subs");
    let cancel = AtomicBool::new(false); // 不可取消 → ck! 永不觸發
    session::ensure_engine_assets(&app, &data, &cancel).await?;
    if !download::any_model_downloaded(&data) {
        download::ensure_model_file(&app, &data, &state.downloading, "turbo", false, session::prog_emit(app.clone(), "model"))
            .await
            .map(|_| ())?;
    }
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_transcription(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    path: String,
    source_kind: String,
    ff_index: Option<i64>,
    playback_url: Option<String>,
    headers: Option<std::collections::BTreeMap<String, String>>,
    duration_sec: f64,
    lang: Option<String>,
    prompt: Option<String>,
    model: String,
    save_srt: bool,
    cache_key_lang: String,
    vad_threshold: f64,
    vad_min_silence_ms: i64,
    overwrite_on_param_change: bool,
) -> Result<(), String> {
    let vad = session::VadParams { threshold: vad_threshold, min_silence_ms: vad_min_silence_ms, vad_enabled: true };
    let source = match source_kind.as_str() {
        "remote" => session::SubSource::Remote {
            playback_url: playback_url.ok_or("remote 來源缺 playback_url")?,
            headers: headers.unwrap_or_default(),
        },
        _ => session::SubSource::Local { ff_index: ff_index.unwrap_or(0) },
    };
    let params = session::SessionParams {
        path, source, duration_sec, lang, prompt, model, save_srt, cache_key_lang, vad,
        overwrite_on_param_change,
    };
    session::start(app, state.inner.clone(), params, state.downloading.clone()).await
}

/// 讀文字檔（字幕載入用）：UTF-8 去 BOM，lossy 解碼。
#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let s = String::from_utf8_lossy(&bytes);
    Ok(s.strip_prefix('\u{feff}').unwrap_or(&s).to_string())
}

#[tauri::command]
pub async fn stop_transcription(state: tauri::State<'_, SubsState>) -> Result<(), String> {
    state.inner.lock().await.shutdown().await;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_loopback_transcription(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    _device_id: Option<String>,
    model: String,
    source_lang: String, // §4.7：前端保證 concrete（非 auto）
    prompt: String,
    vad_threshold: f64,
    vad_min_silence_ms: i64,
) -> Result<(), String> {
    // 舊路徑（T13 前仍需保留）：arm + 立刻 set transcribe params + enable transcribe
    {
        let m = state.inner.lock().await;
        m.set_transcribe_params(session::TranscribeParams {
            model, source_lang, prompt, vad_threshold, vad_min_silence_ms,
        });
        m.set_transcribe(true);
    }
    stream::start(app, state.inner.clone(), crate::capture::source::AudioSource::System, state.downloading.clone()).await
}

#[tauri::command]
pub async fn stop_loopback_transcription(state: tauri::State<'_, SubsState>) -> Result<(), String> {
    state.inner.lock().await.shutdown().await; // 停 task + capture thread + 殺 server + 掃暫存
    Ok(())
}

// ── arm / transcribe 拆分指令（T6；舊 loopback 指令待 T13 移除）────────────────

#[tauri::command]
pub async fn arm_audio_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    source: crate::capture::source::AudioSource,
) -> Result<(), String> {
    let mgr = state.inner.clone();
    let downloading = state.downloading.clone();
    { let m = mgr.lock().await; m.set_transcribe(false); } // 確保 run_loop 以 drain-only 模式啟動
    stream::start(app, mgr, source, downloading).await
}

#[tauri::command]
pub async fn disarm_audio_source(state: tauri::State<'_, SubsState>) -> Result<(), String> {
    let mut m = state.inner.lock().await;
    m.set_transcribe(false);
    m.stop_task_pub(); // abort run_loop + 停 capture thread
    Ok(())
}

#[tauri::command]
pub async fn start_external_transcription(
    state: tauri::State<'_, SubsState>,
    model: String,
    source_lang: String,
    prompt: String,
    vad_threshold: f64,
    vad_min_silence_ms: i64,
) -> Result<(), String> {
    if source_lang == "auto" || source_lang.is_empty() {
        return Err("即時辨識需指定明確語言（非自動）".into());
    }
    let m = state.inner.lock().await;
    m.set_transcribe_params(session::TranscribeParams {
        model, source_lang, prompt, vad_threshold, vad_min_silence_ms,
    });
    m.set_transcribe(true);
    Ok(())
}

#[tauri::command]
pub async fn stop_external_transcription(state: tauri::State<'_, SubsState>) -> Result<(), String> {
    state.inner.lock().await.set_transcribe(false);
    Ok(())
}

#[tauri::command]
pub async fn notify_seek(state: tauri::State<'_, SubsState>, sec: f64) -> Result<(), String> {
    state.inner.lock().await.notify_seek(sec);
    Ok(())
}

/// Path to yt-dlp.exe in the engine dir (None if missing).
fn ytdlp_path(app: &tauri::AppHandle) -> Result<Option<std::path::PathBuf>, String> {
    let data = crate::data_dir(app)?.join("subs");
    Ok(download::find_exe(&data, "yt-dlp.exe"))
}

#[tauri::command]
pub async fn check_ytdlp(app: tauri::AppHandle) -> Result<bool, String> {
    Ok(ytdlp_path(&app)?.is_some())
}

#[tauri::command]
pub async fn provision_ytdlp(app: tauri::AppHandle) -> Result<(), String> {
    let data = crate::data_dir(&app)?.join("subs");
    if download::find_exe(&data, "yt-dlp.exe").is_some() {
        return Ok(());
    }
    download::download_verify(&download::YTDLP, &data, session::prog_emit(app.clone(), "ytdlp"))
        .await
        .map(|_| ())
}

#[tauri::command]
pub async fn resolve_remote(app: tauri::AppHandle, url: String) -> Result<remote::Picked, String> {
    let yt = ytdlp_path(&app)?.ok_or("yt-dlp 尚未安裝")?;
    remote::resolve_remote(&yt, &url).await
}

#[tauri::command]
pub async fn enumerate_playlist(app: tauri::AppHandle, url: String) -> Result<remote::FlatPlaylist, String> {
    let yt = ytdlp_path(&app)?.ok_or("yt-dlp 尚未安裝")?;
    remote::enumerate_playlist(&yt, &url).await
}

#[tauri::command]
pub async fn remote_title(app: tauri::AppHandle, url: String) -> Result<Option<String>, String> {
    let yt = ytdlp_path(&app)?.ok_or("yt-dlp 尚未安裝")?;
    Ok(remote::fetch_remote_title(&yt, &url).await)
}

/// 列出音訊程序來源與輸入裝置（供音源選擇 UI）。COM 須在執行緒上跑 → spawn_blocking。
#[tauri::command]
pub async fn list_audio_sources() -> Result<crate::capture::loopback::AudioSources, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let processes = crate::capture::sessions::list_audio_processes().unwrap_or_default();
        let input_devices = crate::capture::loopback::list_input_devices().unwrap_or_default();
        Ok(crate::capture::loopback::AudioSources { processes, input_devices })
    }).await.map_err(|e| e.to_string())?
}
