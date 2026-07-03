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
pub mod translate;
pub mod silero;

use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use tauri::Emitter;
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

/// 下載所有缺的引擎 runtime（不可取消）。不下載模型（由使用者面板自行下載）。
#[tauri::command]
pub async fn provision_engine(app: tauri::AppHandle) -> Result<(), String> {
    use std::sync::atomic::AtomicBool;
    let data = crate::data_dir(&app)?.join("subs");
    let cancel = AtomicBool::new(false);
    session::ensure_engine_assets(&app, &data, &cancel).await?;
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

// ── arm / transcribe 拆分指令（T6）────────────────────────────────────────────

#[tauri::command]
pub async fn arm_audio_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    source: crate::capture::source::AudioSource,
    record_name: Option<String>,   // Some → 錄製整段擷取音訊到 recordings/<name>
) -> Result<(), String> {
    let mgr = state.inner.clone();
    let downloading = state.downloading.clone();
    { let m = mgr.lock().await; m.set_transcribe(false); } // 確保 run_loop 以 drain-only 模式啟動
    stream::start(app, mgr, source, downloading, record_name).await
}

#[tauri::command]
pub async fn disarm_audio_source(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
) -> Result<(), String> {
    let saved = {
        let mut m = state.inner.lock().await;
        m.set_transcribe(false);
        m.stop_task_pub();        // abort run_loop + 停 capture thread（先停才無並發 append）
        m.finalize_record()       // 收尾錄音（若有）→ 寫 WAV
    };
    if let Some(p) = saved {
        app.emit("recording-saved", p.to_string_lossy().to_string()).ok();
    }
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_external_transcription(
    state: tauri::State<'_, SubsState>,
    model: String,
    source_lang: String,
    prompt: String,
    vad_threshold: f64,
    vad_min_silence_ms: i64,
    target_langs: Vec<String>,
    translate_model: String,
) -> Result<(), String> {
    if source_lang == "auto" || source_lang.is_empty() {
        return Err("即時辨識需指定明確語言（非自動）".into());
    }
    let m = state.inner.lock().await;
    m.set_transcribe_params(session::TranscribeParams {
        model, source_lang, prompt, vad_threshold, vad_min_silence_ms,
        target_langs: target_langs.into_iter().filter(|s| !s.is_empty()).collect(),
        translate_model,
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
pub async fn list_translate_models(app: tauri::AppHandle) -> Result<Vec<ModelStatus>, String> {
    let llm = download::llm_dir(&crate::data_dir(&app)?);
    Ok(download::TRANSLATE_MODEL_KEYS
        .iter()
        .map(|k| ModelStatus { key: k.to_string(), downloaded: download::translate_model_downloaded(&llm, k) })
        .collect())
}

#[tauri::command]
pub async fn translate_engine_ready(app: tauri::AppHandle, key: String) -> Result<bool, String> {
    let llm = download::llm_dir(&crate::data_dir(&app)?);
    Ok(download::llama_server_downloaded(&llm) && download::translate_model_downloaded(&llm, &key))
}

#[tauri::command]
pub async fn check_translate_engine(app: tauri::AppHandle, key: String) -> Result<Vec<MissingAsset>, String> {
    let llm = download::llm_dir(&crate::data_dir(&app)?);
    let hw = tauri::async_runtime::spawn_blocking(hwdetect::detect_hardware_blocking)
        .await
        .map_err(|e| e.to_string())?;
    let mut missing = Vec::new();
    if !download::llama_server_downloaded(&llm) {
        let sz = if hw.backend == "cuda" { download::SIZE_MB_LLAMA_CUDA } else { download::SIZE_MB_LLAMA_CPU };
        missing.push(MissingAsset { kind: "llmServer".into(), size_mb: sz });
    }
    if !download::translate_model_downloaded(&llm, &key) {
        missing.push(MissingAsset { kind: "llmModel".into(), size_mb: download::translate_model_size_mb(&key) });
    }
    Ok(missing)
}

/// 裝翻譯執行套件（llama-server + CUDA cudart）；不下載模型。進度事件 key = "llm-runtime"。
#[tauri::command]
pub async fn provision_translate_runtime(app: tauri::AppHandle, state: tauri::State<'_, SubsState>) -> Result<(), String> {
    let llm = download::llm_dir(&crate::data_dir(&app)?);
    let hw = tauri::async_runtime::spawn_blocking(hwdetect::detect_hardware_blocking).await.map_err(|e| e.to_string())?;
    let backend = if hw.backend == "cuda" { hwdetect::Backend::Cuda } else { hwdetect::Backend::Cpu };
    let ap = app.clone();
    let on_prog = move |done: u64, total: Option<u64>| {
        let _ = ap.emit("model-download", ModelDownloadEvent {
            key: "llm-runtime".into(), phase: "downloading".into(), done, total, message: None });
    };
    match download::ensure_llm_runtime(&app, &llm, &state.downloading, backend, on_prog).await {
        Ok(_) => { let _ = app.emit("model-download", ModelDownloadEvent { key: "llm-runtime".into(), phase: "done".into(), done: 0, total: None, message: None }); Ok(()) }
        Err(e) => { let _ = app.emit("model-download", ModelDownloadEvent { key: "llm-runtime".into(), phase: "error".into(), done: 0, total: None, message: Some(e.clone()) }); Err(e) }
    }
}

/// 只下載翻譯 GGUF 模型；進度事件 key = translate key（TranslatePanel 進度條已綁）。
#[tauri::command]
pub async fn download_translate_model(app: tauri::AppHandle, state: tauri::State<'_, SubsState>, key: String) -> Result<(), String> {
    let llm = download::llm_dir(&crate::data_dir(&app)?);
    let (ap, kp) = (app.clone(), key.clone());
    let on_prog = move |done: u64, total: Option<u64>| {
        let _ = ap.emit("model-download", ModelDownloadEvent {
            key: kp.clone(), phase: "downloading".into(), done, total, message: None });
    };
    match download::ensure_llm_model(&app, &llm, &state.downloading, &key, on_prog).await {
        Ok(_) => { let _ = app.emit("model-download", ModelDownloadEvent { key: key.clone(), phase: "done".into(), done: 0, total: None, message: None }); Ok(()) }
        Err(e) => { let _ = app.emit("model-download", ModelDownloadEvent { key: key.clone(), phase: "error".into(), done: 0, total: None, message: Some(e.clone()) }); Err(e) }
    }
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

// ── translate_cues 指令（Task 4）────────────────────────────────────────────

use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XlateCueIn {
    pub id: String,
    pub source_text: String,
    pub source_lang: Option<String>,
    pub start_sec: f64,
    pub end_sec: f64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct XlateCueOut {
    pub id: String,
    pub translations: BTreeMap<String, String>,
}

/// 純翻譯批次：每 cue × 每 target 呼 translator；回 (輸出[id 回聲], 待落地 recs by target)。
/// 空回（LocalLlmTranslator 已對 is_untranslated 回空）→ 不入 translations、不入 recs。
pub async fn translate_batch(
    tr: &dyn translate::Translator,
    cues: &[XlateCueIn],
    targets: &[String],
) -> (Vec<XlateCueOut>, std::collections::HashMap<String, Vec<cache::XlateRec>>) {
    let mut outs = Vec::with_capacity(cues.len());
    let mut recs: std::collections::HashMap<String, Vec<cache::XlateRec>> = std::collections::HashMap::new();
    for c in cues {
        let mut translations = BTreeMap::new();
        for tgt in targets {
            match tr.translate(&c.source_text, c.source_lang.as_deref(), tgt).await {
                Ok(t) if !t.is_empty() => {
                    recs.entry(tgt.clone()).or_default().push(cache::XlateRec {
                        start_sec: c.start_sec,
                        end_sec: c.end_sec,
                        text: t.clone(),
                    });
                    translations.insert(tgt.clone(), t);
                }
                Ok(_) => {} // 未翻/echo → 略過（顯原文）
                Err(e) => eprintln!("[translate_cues] {tgt} 翻譯失敗：{e}"),
            }
        }
        outs.push(XlateCueOut { id: c.id.clone(), translations });
    }
    (outs, recs)
}

/// clock 模式排程器呼叫：確保 translator → 批翻 → （save_srt 時）落地 sidecar → 回傳譯文。
/// 併發限制交由前端小批 + in-flight cap；此指令本身可與另一併發呼叫序列化於 upsert 鎖。
#[tauri::command]
pub async fn translate_cues(
    app: tauri::AppHandle,
    state: tauri::State<'_, SubsState>,
    source: cache::XlateSource,
    cues: Vec<XlateCueIn>,
    targets: Vec<String>,
    translate_model: String,
    save_srt: bool,
) -> Result<Vec<XlateCueOut>, String> {
    if targets.is_empty() || cues.is_empty() {
        return Ok(Vec::new());
    }
    let data = crate::data_dir(&app)?;
    // ensure_translator 期待 data=<root>/subs（內部取 parent 為 root 對齊 llm/）
    let tr = session::ensure_translator(
        &app,
        &state.inner,
        &data.join("subs"),
        &state.downloading,
        &translate_model,
    )
    .await?;
    let (outs, recs) = translate_batch(&*tr, &cues, &targets).await;
    if save_srt {
        let m = state.inner.lock().await;
        for (tgt, rs) in recs {
            m.upsert_translations(&data, &source, &tgt, rs);
        }
    }
    Ok(outs)
}

// ── read_cued_translations 指令（Task 5）────────────────────────────────────────────

/// 多 target 的 start-ms map → 每 start-ms 一個 XlateCueOut（id=start-ms 字串）。純函式。
pub fn collate_translation_cues(
    per_target: Vec<(String, BTreeMap<i64, cache::XlateRec>)>,
) -> Vec<XlateCueOut> {
    let mut by_ms: BTreeMap<i64, BTreeMap<String, String>> = BTreeMap::new();
    for (tgt, map) in per_target {
        for (ms, rec) in map {
            by_ms.entry(ms).or_default().insert(tgt.clone(), rec.text);
        }
    }
    by_ms.into_iter().map(|(ms, translations)| XlateCueOut { id: ms.to_string(), translations }).collect()
}

/// cue 就緒時瞬載已快取譯文（live seed / file load 後呼）。id=start-ms 字串，前端時間對齊 merge。
#[tauri::command]
pub async fn read_cued_translations(
    app: tauri::AppHandle,
    source: cache::XlateSource,
    targets: Vec<String>,
) -> Result<Vec<XlateCueOut>, String> {
    let root = crate::data_dir(&app)?;
    let per_target: Vec<(String, BTreeMap<i64, cache::XlateRec>)> = targets
        .into_iter()
        .map(|tgt| {
            let path = cache::translation_sidecar_path(&root, &source, &tgt);
            (tgt, cache::read_translation_srt(&path))
        })
        .collect();
    Ok(collate_translation_cues(per_target))
}

#[cfg(test)]
mod xlate_tests {
    use super::*;
    use crate::subs::translate::Translator;
    use async_trait::async_trait;

    #[test]
    fn collate_sidecars_by_startms() {
        // 純併合邏輯抽出可測（見 Step 3 collate_translation_cues）
        use crate::subs::cache::XlateRec;
        let mut a = std::collections::BTreeMap::new();
        a.insert(1000_i64, XlateRec { start_sec: 1.0, end_sec: 2.0, text: "你好".into() });
        let mut b = std::collections::BTreeMap::new();
        b.insert(1000_i64, XlateRec { start_sec: 1.0, end_sec: 2.0, text: "こんにちは".into() });
        let outs = collate_translation_cues(vec![("zh-Hant".to_string(), a), ("ja".to_string(), b)]);
        assert_eq!(outs.len(), 1);
        assert_eq!(outs[0].id, "1000"); // start-ms 字串
        assert_eq!(outs[0].translations["zh-Hant"], "你好");
        assert_eq!(outs[0].translations["ja"], "こんにちは");
    }

    struct MockTr; // 回 "<t>:<src>"；text 為 "echo" 時回空模擬 is_untranslated
    #[async_trait]
    impl Translator for MockTr {
        async fn translate(
            &self,
            text: &str,
            source_lang: Option<&str>,
            target_lang: &str,
        ) -> Result<String, String> {
            if text == "echo" {
                return Ok(String::new()); // 模擬未翻→空
            }
            Ok(format!("{target_lang}:{}", source_lang.unwrap_or("none")))
        }
    }

    #[tokio::test]
    async fn translate_batch_maps_and_skips_empty() {
        let cues = vec![
            XlateCueIn {
                id: "1000".into(),
                source_text: "hi".into(),
                source_lang: Some("ja".into()),
                start_sec: 1.0,
                end_sec: 2.0,
            },
            XlateCueIn {
                id: "2000".into(),
                source_text: "echo".into(),
                source_lang: None,
                start_sec: 2.0,
                end_sec: 3.0,
            },
        ];
        let (outs, recs) = translate_batch(&MockTr, &cues, &["zh-Hant".into()]).await;
        // id 回聲、非空 translations
        assert_eq!(outs[0].id, "1000");
        assert_eq!(outs[0].translations["zh-Hant"], "zh-Hant:ja");
        // echo → 空 → 該 cue 無 translations（out 仍在但 map 空）、不入 recs
        assert!(outs[1].translations.is_empty());
        assert_eq!(recs["zh-Hant"].len(), 1); // 只有第一條進快取
        assert_eq!(recs["zh-Hant"][0].text, "zh-Hant:ja");
    }
}
