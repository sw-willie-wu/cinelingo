use super::{audio, cache, download, hallucination, hwdetect, merge, plan, remote, vad, whisper, ProgressEvent, SessionResetEvent};
use super::cue::Cue;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// VAD 啟動參數（whisper-server 啟動旗標；改值需重啟 server）。
#[derive(Clone, Debug)]
pub struct VadParams {
    pub threshold: f64,
    pub min_silence_ms: i64,
    pub vad_enabled: bool, // false = 串流路徑（--vad OFF，避 0-speech 崩潰）
}

/// threshold 為 f64 → 用容差比較，避免浮點雜訊造成無謂重啟。
pub fn vad_eq(a: &VadParams, b: &VadParams) -> bool {
    a.vad_enabled == b.vad_enabled
        && (a.threshold - b.threshold).abs() < 1e-4
        && a.min_silence_ms == b.min_silence_ms
}

/// server 是否可重用：model 相同且 VAD 相同（含容差）才可。
/// loaded_vad 為 None（不該發生）一律視為不可重用。
pub fn server_reusable(
    loaded_model: Option<&str>,
    loaded_vad: Option<&VadParams>,
    want_model: &str,
    want_vad: &VadParams,
) -> bool {
    loaded_model == Some(want_model)
        && loaded_vad.map(|lv| vad_eq(lv, want_vad)).unwrap_or(false)
}

/// 字幕音訊來源：local＝檔+ff_index；remote＝從 muxed playback_url 抽（帶 headers）。
pub enum SubSource {
    Local { ff_index: i64 },
    Remote { playback_url: String, headers: BTreeMap<String, String> },
}

/// Per-session 資料輸入包（純資料；AppHandle / 狀態 handle / cancel token 另傳）。
pub struct SessionParams {
    pub path: String,
    pub source: SubSource,
    pub duration_sec: f64,
    pub lang: Option<String>,
    pub prompt: Option<String>,
    pub model: String,
    pub save_srt: bool,
    pub cache_key_lang: String,
    pub vad: VadParams,
    pub overwrite_on_param_change: bool,
}

#[derive(Default)]
pub struct Manager {
    counter: u64,
    server: Option<whisper::WhisperServer>,
    loaded_model: Option<String>,
    loaded_vad: Option<VadParams>,
    ffmpeg: Option<PathBuf>,
    http: Option<reqwest::Client>,
    task: Option<tauri::async_runtime::JoinHandle<()>>,
    seek_to: Arc<std::sync::Mutex<Option<f64>>>,
    cancel: Arc<AtomicBool>,
    data_dir: Option<PathBuf>,
    capture_stop: Option<std::sync::Arc<AtomicBool>>,
    capture_thread: Option<std::thread::JoinHandle<()>>,
}

/// 刪除 data 內所有 track-*.wav / win-*.wav 暫存（best-effort）。
/// 因 stop_task 以 abort 收尾，run 的「迴圈後清理」不會執行 → 改在 session 開始 + shutdown 時掃。
fn sweep_temp(dir: &std::path::Path) {
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name();
            let Some(n) = name.to_str() else { continue };
            if (n.starts_with("track-") || n.starts_with("win-")) && n.ends_with(".wav") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

impl Manager {
    /// 同步停舊任務（abort，不 await → 不在鎖內等任務 → 無死鎖）。server 與任務解耦不受影響。
    fn stop_task(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(h) = self.task.take() {
            h.abort();
        }
        // 停 WASAPI 擷取執行緒（loopback session）：signal flag + join（該執行緒每 ~200ms 查旗標、快速退出；不持 Manager 鎖故不死鎖）。
        if let Some(flag) = self.capture_stop.take() {
            flag.store(true, Ordering::SeqCst);
        }
        if let Some(h) = self.capture_thread.take() {
            let _ = h.join();
        }
    }

    /// 完全停用：停任務 + 殺 server + 掃暫存。
    pub async fn shutdown(&mut self) {
        self.stop_task();
        self.loaded_model = None;
        self.loaded_vad = None;
        if let Some(s) = self.server.take() {
            s.kill().await;
        }
        if let Some(d) = &self.data_dir {
            sweep_temp(d);
        }
    }

    pub fn notify_seek(&self, sec: f64) {
        *self.seek_to.lock().unwrap() = Some(sec);
    }

    /// 作廢目前 server（崩潰退路用）：丟 handle（kill_on_drop 連帶終止）、清 loaded 狀態 →
    /// 下次 ensure_server 視為不可重用 → 重啟。
    pub fn invalidate_server(&mut self) {
        self.server = None;
        self.loaded_model = None;
        self.loaded_vad = None;
    }

    /// loopback 啟動時存擷取執行緒 handle，供 stop_task/shutdown 收尾。
    #[allow(dead_code)]
    pub(crate) fn set_capture(&mut self, flag: std::sync::Arc<AtomicBool>, thread: std::thread::JoinHandle<()>) {
        self.capture_stop = Some(flag);
        self.capture_thread = Some(thread);
    }

    // subs::stream（loopback 串流）跨模組存取 Manager 內部所需的 pub(crate) 包裝。
    pub(crate) fn stop_task_pub(&mut self) { self.stop_task(); }
    pub(crate) fn bump_counter(&mut self) { self.counter += 1; }
    pub(crate) fn session_id_str(&self) -> String { format!("s{}", self.counter) }
    pub(crate) fn reset_cancel(&mut self) { self.cancel = Arc::new(AtomicBool::new(false)); }
    pub(crate) fn cancel_arc(&self) -> Arc<AtomicBool> { self.cancel.clone() }
    pub(crate) fn set_data_dir(&mut self, d: std::path::PathBuf) { self.data_dir = Some(d); }
    pub(crate) fn set_task(&mut self, h: tauri::async_runtime::JoinHandle<()>) { self.task = Some(h); }
    pub(crate) fn http_clone(&self) -> Option<reqwest::Client> { self.http.clone() }
}

pub async fn start(
    app: AppHandle,
    mgr: Arc<tokio::sync::Mutex<Manager>>,
    params: SessionParams,
    downloading: Arc<std::sync::Mutex<HashSet<String>>>,
) -> Result<(), String> {
    let data = crate::data_dir(&app)?.join("subs");
    let app2 = app.clone();
    let mgr2 = mgr.clone();
    {
        let mut m = mgr.lock().await;
        m.stop_task(); // abort 舊（同步）
        m.counter += 1;
        let session_id = format!("s{}", m.counter);
        *m.seek_to.lock().unwrap() = None;
        m.cancel = Arc::new(AtomicBool::new(false));
        m.data_dir = Some(data.clone());
        let cancel = m.cancel.clone();
        let seek_to = m.seek_to.clone();
        app.emit("sub-session-reset", SessionResetEvent { session_id: session_id.clone(), no_clock: false })
            .ok();
        // spawn + 存 handle 同一臨界區 → 無漏接 race（spawn 只排程，task 待本鎖釋放後才跑）
        let handle = tauri::async_runtime::spawn(async move {
            if let Err(e) = run(
                app2.clone(), mgr2, session_id, params, seek_to, cancel, data, downloading,
            )
            .await
            {
                app2.emit(
                    "sub-progress",
                    ProgressEvent { phase: "error".into(), done: 0, total: None, message: e },
                )
                .ok();
            }
        });
        m.task = Some(handle);
    }
    Ok(())
}

/// 確保資產 + 起/重用 server（模型不同則換掉），回 port。重活在鎖外做。
pub(crate) async fn ensure_server(
    app: &AppHandle,
    mgr: &Arc<tokio::sync::Mutex<Manager>>,
    data: &std::path::Path,
    cancel: &AtomicBool,
    model: &str,
    vad_params: &VadParams,
    downloading: &Arc<std::sync::Mutex<HashSet<String>>>,
) -> Result<u16, String> {
    // fast path：server 已起且同模型同 VAD
    {
        let m = mgr.lock().await;
        if let Some(s) = &m.server {
            if server_reusable(m.loaded_model.as_deref(), m.loaded_vad.as_ref(), model, vad_params) {
                return Ok(s.port);
            }
        }
    }
    let (exe, model_path, vad, ffmpeg) = ensure_assets(app, data, cancel, model, downloading).await?;
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    app.emit(
        "sub-progress",
        ProgressEvent { phase: "start".into(), done: 0, total: None, message: "啟動引擎".into() },
    )
    .ok();
    let server = whisper::WhisperServer::start(&exe, &model_path, &vad, vad_params).await?;
    let port = server.port;
    let old = {
        let mut m = mgr.lock().await;
        if m.http.is_none() {
            m.http = Some(reqwest::Client::new());
        }
        m.ffmpeg = Some(ffmpeg);
        // 競態：provision 期間別人已起同模型同 VAD → 用既有、棄自己這顆
        if let Some(s) = &m.server {
            if server_reusable(m.loaded_model.as_deref(), m.loaded_vad.as_ref(), model, vad_params) {
                let p = s.port;
                drop(m);
                server.kill().await;
                return Ok(p);
            }
        }
        let old = m.server.take(); // 不同模型/VAD 的舊 server
        m.server = Some(server);
        m.loaded_model = Some(model.to_string());
        m.loaded_vad = Some(vad_params.clone());
        old
    };
    if let Some(old) = old {
        old.kill().await; // 殺舊（鎖外）
    }
    Ok(port)
}

#[allow(clippy::too_many_arguments)]
async fn run(
    app: AppHandle,
    mgr: Arc<tokio::sync::Mutex<Manager>>,
    session_id: String,
    params: SessionParams,
    seek_to: Arc<std::sync::Mutex<Option<f64>>>,
    cancel: Arc<AtomicBool>,
    data: PathBuf,
    downloading: Arc<std::sync::Mutex<HashSet<String>>>,
) -> Result<(), String> {
    let SessionParams { path, source, duration_sec, lang, prompt, model, save_srt, cache_key_lang, vad, overwrite_on_param_change } = params;
    if duration_sec <= 0.0 {
        app.emit(
            "sub-progress",
            ProgressEvent { phase: "error".into(), done: 0, total: None, message: "無法取得影片長度，請重試".into() },
        )
        .ok();
        return Ok(());
    }
    sweep_temp(&data); // 清前一個 session 殘留的暫存
    // Part C 種子：在 ensure_server（server 開機/下載）之前讀快取 → emit → 秒顯示。
    // 快取根目錄＝app_data（data.parent()），不是 data(=app_data/subs)。
    let app_data = data.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| data.clone());
    let srt_path = cache::cache_path_for(&app_data, &path, &cache_key_lang);
    let json_path = srt_path.with_extension("json");
    let cache_params = cache::CacheParams {
        model: &model,
        vad_threshold: vad.threshold,
        vad_min_silence_ms: vad.min_silence_ms,
    };
    let mut covered: plan::Intervals = Vec::new();
    let mut emitted: HashSet<String> = HashSet::new();
    let mut all_cues: Vec<Cue> = Vec::new();
    if save_srt {
        let meta = cache::read_cached_meta(&json_path);
        // A3：覆寫開關 ON 且快取參數與目前不同（或舊快取無標記）→ stale，跳過 seed → 全片重轉、write_cache 覆寫舊檔。
        let stale = overwrite_on_param_change
            && meta
                .as_ref()
                .is_none_or(|m| !cache::cache_params_match(m, &cache_params));
        if !stale {
            let seed: Vec<Cue> = cache::read_cached_cues(&srt_path, &session_id)
                .into_iter()
                .filter(|c| !hallucination::is_boilerplate(&c.source_text))
                .collect(); // 已蓋當前 sessionId + 濾掉既有 boilerplate 幻覺（重開舊快取片即隱藏）
            if !seed.is_empty() {
                for c in &seed {
                    emitted.insert(c.id.clone());
                }
                all_cues = seed.clone();
                app.emit("sub-cue-batch", seed).ok(); // 單一批次事件，前端 bulk upsert（N2）
            }
            let cov = meta.map(|m| m.coverage).unwrap_or_default();
            if !cov.is_empty() {
                for (s, e) in cov {
                    covered = plan::add_interval(covered, s, e);
                }
            } else {
                // §6.6 退路：只有 .srt 無 .json → 用 cue 區間當保守 coverage。
                for c in &all_cues {
                    covered = plan::add_interval(covered, c.start_sec, c.end_sec);
                }
            }
        }
    }
    // 已快取片：把轉寫前緣推到覆蓋邊界，讓前端不在已覆蓋區誤顯「(轉錄中)」。
    // sub-session-reset（start() 發）已把前端 frontier 重設為 0；全覆蓋時迴圈永不發 transcribe → 這裡補一發。
    if !covered.is_empty() {
        let frontier = plan::first_uncovered(&covered, 0.0, duration_sec).unwrap_or(duration_sec);
        app.emit(
            "sub-progress",
            ProgressEvent {
                phase: "transcribe".into(),
                done: frontier as u64,
                total: Some(duration_sec as u64),
                message: String::new(),
            },
        )
        .ok();
    }
    let mut port = ensure_server(&app, &mgr, &data, &cancel, &model, &vad, &downloading).await?;
    let (ffmpeg, http) = {
        let m = mgr.lock().await;
        (m.ffmpeg.clone().ok_or("no ffmpeg")?, m.http.clone().ok_or("no http")?)
    };
    app.emit(
        "sub-progress",
        ProgressEvent { phase: "decode".into(), done: 0, total: None, message: "解碼音訊".into() },
    )
    .ok();
    let track_wav = data.join(format!("track-{session_id}.wav"));
    match &source {
        SubSource::Local { ff_index } => {
            audio::decode_track(&ffmpeg, &path, *ff_index, &track_wav).await?;
        }
        SubSource::Remote { playback_url, headers } => {
            // 從 muxed 抽音訊（不限速、整支秒級）；失敗 → 以 watch_url(=path) re-resolve 重試一次。
            let yt = download::find_exe(&data, "yt-dlp.exe").ok_or("yt-dlp.exe not found")?;
            remote::decode_remote_with_retry(&ffmpeg, &yt, &path, playback_url, headers, &track_wav).await?;
        }
    }
    // 整軌預先 VAD：拿語音時間軸 → loop 跳過非語音視窗，避免 whisper-server 對 0-speech 視窗崩潰。
    // 偵測失敗/工具缺 → None → 不跳窗（退回現狀），由 transcribe 失敗退路兜底。
    let speech: Option<Vec<(f64, f64)>> = {
        let vad_model = data.join(download::MODEL_VAD.filename);
        match download::find_exe(&data.join("whisper"), "whisper-vad-speech-segments.exe") {
            Some(vad_exe) => match vad::detect_speech_segments(
                &vad_exe, &vad_model, &track_wav, vad.threshold, vad.min_silence_ms,
            )
            .await
            {
                Ok(s) => Some(s),
                Err(e) => {
                    eprintln!("[subs] VAD 預掃失敗，改不跳窗：{e}");
                    None
                }
            },
            None => {
                eprintln!("[subs] 找不到 whisper-vad-speech-segments.exe，改不跳窗");
                None
            }
        }
    };
    let win_wav = data.join(format!("win-{session_id}.wav"));
    let mut lang_used = lang;
    while !cancel.load(Ordering::SeqCst) {
        let seek = { seek_to.lock().unwrap().take() };
        // 挑下一個未覆蓋點：seek→從 seek 點；否則/或 seek 點已覆蓋→最早 gap。
        let point = match seek {
            Some(s) => plan::first_uncovered(&covered, s, duration_sec)
                .or_else(|| plan::first_uncovered(&covered, 0.0, duration_sec)),
            None => plan::first_uncovered(&covered, 0.0, duration_sec),
        };
        let Some(point) = point else {
            // 全覆蓋→idle 等 seek
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            continue;
        };
        let start = (point - plan::OVERLAP_SEC).max(0.0);
        let max_end = (point + plan::WINDOW_SEC).min(duration_sec);
        // 窗尾對齊 VAD 靜音縫（不切穿語音段）；VAD 不可用 → 退回固定窗（現狀）。
        let end = match &speech {
            Some(sp) => plan::aligned_window_end(sp, point, max_end),
            None => max_end,
        };
        // 非語音視窗：不送會崩的 server，直接標記已覆蓋、推進 frontier。
        if let Some(sp) = &speech {
            if !vad::window_has_speech(sp, start, end) {
                covered = plan::add_interval(covered, start, end);
                let frontier = plan::first_uncovered(&covered, 0.0, duration_sec).unwrap_or(duration_sec);
                app.emit(
                    "sub-progress",
                    ProgressEvent { phase: "transcribe".into(), done: frontier as u64, total: Some(duration_sec as u64), message: String::new() },
                )
                .ok();
                continue;
            }
        }
        audio::slice_window(&ffmpeg, &track_wav, start, end, &win_wav).await?;
        let (segs, detected) = match whisper::transcribe(&http, port, &win_wav, lang_used.as_deref(), prompt.as_deref()).await {
            Ok(r) => r,
            Err(e) => {
                // server 可能在此視窗崩潰退出 → 標記已覆蓋（避免重選同窗、確保前進）、作廢死 server、重啟、續跑。
                eprintln!("[subs] transcribe 失敗（server 可能崩潰），重啟續跑：{e}");
                covered = plan::add_interval(covered, start, end);
                {
                    mgr.lock().await.invalidate_server();
                }
                port = ensure_server(&app, &mgr, &data, &cancel, &model, &vad, &downloading).await?;
                let frontier = plan::first_uncovered(&covered, 0.0, duration_sec).unwrap_or(duration_sec);
                app.emit(
                    "sub-progress",
                    ProgressEvent { phase: "transcribe".into(), done: frontier as u64, total: Some(duration_sec as u64), message: String::new() },
                )
                .ok();
                continue;
            }
        };
        if lang_used.is_none() {
            lang_used = detected;
        }
        let mut cues = merge::merge_segments(&session_id, lang_used.as_deref(), start, &segs);
        cues.retain(|c| !plan::fully_covers(&covered, c.start_sec, c.end_sec));
        cues.retain(|c| !hallucination::is_boilerplate(&c.source_text)); // 濾掉 boilerplate 幻覺（字幕組署名/訂閱）
        covered = plan::add_interval(covered, start, end);
        for c in cues {
            if emitted.insert(c.id.clone()) {
                if save_srt {
                    all_cues.push(c.clone());
                }
                app.emit("sub-cue", c).ok();
            }
        }
        if save_srt {
            let _ = cache::write_cache(
                &srt_path, &json_path, &all_cues, &covered, &path, duration_sec, lang_used.as_deref(),
                &cache_params,
            );
        }
        let frontier = plan::first_uncovered(&covered, 0.0, duration_sec).unwrap_or(duration_sec);
        app.emit(
            "sub-progress",
            ProgressEvent {
                phase: "transcribe".into(),
                done: frontier as u64,
                total: Some(duration_sec as u64),
                message: String::new(),
            },
        )
        .ok();
    }
    // 注意：以 abort 收尾時這裡不會執行 → 暫存清理改由 sweep_temp（session 開始 + shutdown）負責。
    Ok(())
}

/// emit `sub-progress` 進度回呼工廠（model/vad/backend/ffmpeg 共用；provision 亦用）。
/// pub(crate)：provision_engine（mod.rs）需呼叫。
pub(crate) fn prog_emit(app: AppHandle, phase: &'static str) -> impl FnMut(u64, Option<u64>) {
    move |done: u64, total: Option<u64>| {
        app.emit(
            "sub-progress",
            ProgressEvent { phase: phase.into(), done, total, message: String::new() },
        )
        .ok();
    }
}

/// 確保引擎三件（VAD / 加速後端 / ffmpeg）就緒，缺才下載解壓；emit `sub-progress`。
/// 回 (whisper-server.exe, vad, ffmpeg.exe)。pub(crate)：provision_engine 需呼叫。
pub(crate) async fn ensure_engine_assets(
    app: &AppHandle,
    data: &std::path::Path,
    cancel: &AtomicBool,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    use download::*;
    tokio::fs::create_dir_all(data).await.map_err(|e| e.to_string())?;
    let vad = data.join(MODEL_VAD.filename);
    let whisper_dir = data.join("whisper");
    let ffmpeg_dir = data.join("ffmpeg");
    let backend = hwdetect::pick_backend(&hwdetect::detect_adapters());
    macro_rules! ck {
        () => {
            if cancel.load(Ordering::SeqCst) {
                return Err("cancelled".into());
            }
        };
    }
    ck!();
    if !vad.exists() {
        download_verify(&MODEL_VAD, data, prog_emit(app.clone(), "vad")).await?;
    }
    ck!();
    if find_exe(&whisper_dir, "whisper-server.exe").is_none() {
        let z = download_verify(&backend_asset(backend), data, prog_emit(app.clone(), "backend")).await?;
        unzip(&z, &whisper_dir)?;
        let _ = tokio::fs::remove_file(&z).await;
    }
    ck!();
    if find_exe(&ffmpeg_dir, "ffmpeg.exe").is_none() {
        let z = download_verify(&FFMPEG, data, prog_emit(app.clone(), "ffmpeg")).await?;
        unzip(&z, &ffmpeg_dir)?;
        let _ = tokio::fs::remove_file(&z).await;
    }
    let exe = find_exe(&whisper_dir, "whisper-server.exe").ok_or("whisper-server.exe not found")?;
    let ffmpeg = find_exe(&ffmpeg_dir, "ffmpeg.exe").ok_or("ffmpeg.exe not found")?;
    Ok((exe, vad, ffmpeg))
}

/// 確保資產就緒（缺才下載解壓）。回 (whisper-server.exe, 選定模型, vad, ffmpeg.exe)。
/// 模型最先下載（維持原 sub-progress phase 順序 model→vad→backend→ffmpeg），再引擎三件。
async fn ensure_assets(
    app: &AppHandle,
    data: &std::path::Path,
    cancel: &AtomicBool,
    model: &str,
    downloading: &Arc<std::sync::Mutex<HashSet<String>>>,
) -> Result<(PathBuf, PathBuf, PathBuf, PathBuf), String> {
    if cancel.load(Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    let model_path =
        download::ensure_model_file(app, data, downloading, model, false, prog_emit(app.clone(), "model")).await?;
    let (exe, vad, ffmpeg) = ensure_engine_assets(app, data, cancel).await?;
    Ok((exe, model_path, vad, ffmpeg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_eq_equal_and_within_tolerance() {
        let a = VadParams { threshold: 0.50, min_silence_ms: 100, vad_enabled: true };
        let b = VadParams { threshold: 0.50, min_silence_ms: 100, vad_enabled: true };
        assert!(vad_eq(&a, &b));
        // threshold 容差內（< 1e-4）視為相等
        let c = VadParams { threshold: 0.50001, min_silence_ms: 100, vad_enabled: true };
        assert!(vad_eq(&a, &c));
    }

    #[test]
    fn vad_eq_threshold_outside_tolerance() {
        let a = VadParams { threshold: 0.50, min_silence_ms: 100, vad_enabled: true };
        let b = VadParams { threshold: 0.55, min_silence_ms: 100, vad_enabled: true };
        assert!(!vad_eq(&a, &b));
    }

    #[test]
    fn vad_eq_min_silence_differs() {
        let a = VadParams { threshold: 0.50, min_silence_ms: 100, vad_enabled: true };
        let b = VadParams { threshold: 0.50, min_silence_ms: 200, vad_enabled: true };
        assert!(!vad_eq(&a, &b));
    }

    #[test]
    fn server_reusable_same_model_same_vad() {
        let vad = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        assert!(server_reusable(Some("turbo"), Some(&vad), "turbo", &vad));
    }

    #[test]
    fn server_reusable_same_model_diff_vad() {
        let loaded = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        let want = VadParams { threshold: 0.7, min_silence_ms: 100, vad_enabled: true };
        assert!(!server_reusable(Some("turbo"), Some(&loaded), "turbo", &want));
    }

    #[test]
    fn server_reusable_diff_model() {
        let vad = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        assert!(!server_reusable(Some("small"), Some(&vad), "turbo", &vad));
    }

    #[test]
    fn server_reusable_no_server() {
        let vad = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        assert!(!server_reusable(None, None, "turbo", &vad));
        // 有 model 但無 loaded_vad（理論上不會發生，仍應為 false）
        assert!(!server_reusable(Some("turbo"), None, "turbo", &vad));
    }

    #[test]
    fn vad_eq_enabled_flag_differs() {
        let on = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        let off = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: false };
        assert!(!vad_eq(&on, &off)); // ON↔OFF 必判不同 → 強制重啟 server
    }
}
