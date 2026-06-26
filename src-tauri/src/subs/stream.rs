#![allow(dead_code)]

use super::cue::{derive_cue_id, Cue};
use super::{hallucination, session, whisper, ProgressEvent, SessionResetEvent};
use crate::capture::loopback;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

// 靜音路徑保留的尾段秒數（純靜音輪保留此秒長的尾段，供下一輪 speech gate 0.3s 判斷）：
// 必 >0，否則靜音 trim 清空整個 buffer，每輪只進 ~0.09s 又被清掉
// → 永遠到不了 speech gate(0.3s) → 卡死在靜音路徑（實機 log 證實）。
pub const SILENCE_KEEP_SEC: f64 = 1.0;
pub const MIN_SPEECH_BUFFER_SEC: f64 = 0.3; // 須累積這麼多音訊才開始轉寫（speech gate 下限）

// ── collapse_repetition 常數（§3.2/§5）──
pub const COLLAPSE_MIN_UNIT_CHARS: usize = 2; // 最短收摺單元字數（保護單字疊字）
pub const COLLAPSE_MIN_REPEATS: usize = 3;    // 連續次數門檻
pub const COLLAPSE_KEEP: usize = 1;           // 收摺後保留份數
pub const MAX_PERIOD_CHARS: usize = 8;        // 週期搜尋上界

/// 收摺「連續重複的重複幻覺牆」（賣掉×18、謝謝大家×3）。在每個位置取「覆蓋最廣」的重複：
/// 對 p∈[1,MAX_PERIOD_CHARS] 算 r_p（以 chars[i..i+p] 為單元、從 i 起連續相等的塊數），
/// 取 coverage=p*r_p 最大、同覆蓋取**最小 p**（保護單字疊字：全等串 tie 取 p0=1<MIN_UNIT 不收）。
/// 收摺 iff p0>=COLLAPSE_MIN_UNIT_CHARS ∧ r>=COLLAPSE_MIN_REPEATS。決定性、純函式。
pub fn collapse_repetition(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let max_p = MAX_PERIOD_CHARS.min(n - i);
        // 取覆蓋最廣 (p*r) 的重複；升序遍歷 → 同覆蓋保留最小 p
        let mut best: Option<(usize, usize)> = None; // (p, r)
        for p in 1..=max_p {
            let mut r = 1usize;
            loop {
                let s = i + r * p;
                if s + p > n || chars[i..i + p] != chars[s..s + p] { break; }
                r += 1;
            }
            if r >= 2 {
                let cov = p * r;
                match best {
                    Some((bp, br)) if bp * br >= cov => {} // 既有覆蓋 ≥ → 保留（小 p 優先）
                    _ => best = Some((p, r)),
                }
            }
        }
        match best {
            Some((p0, r)) if p0 >= COLLAPSE_MIN_UNIT_CHARS && r >= COLLAPSE_MIN_REPEATS => {
                out.extend_from_slice(&chars[i..i + p0 * COLLAPSE_KEEP]);
                i += p0 * r;
            }
            _ => { out.push(chars[i]); i += 1; }
        }
    }
    out.into_iter().collect()
}

// ── 串流定稿常數（§5）──
pub const NO_SPEECH_COMMIT: f64 = 0.45;     // commit gate（§3.3）
pub const AGREEMENT_THRESH: u32 = 5;        // T2 末段(collapsed)不變輪數
pub const FORCE_COMMIT_SEC: f64 = 30.0;     // T4 安全 cap
pub const FINALIZE_SILENCE_SEC: f64 = 0.7;  // T3 句末靜音門檻（遲滯、獨立於短 gate）
// SILENCE_KEEP_SEC / MIN_SPEECH_BUFFER_SEC 沿用既有

// 回灌近期已定稿文字當 prompt（對齊 WhisperLive condition_on_previous_text）：上限字元數。
// whisper prompt ~224 token 上限，CJK 約 1+ token/字 → 取保守值，超過從前端 trim。
pub const RECENT_CONTEXT_CHARS: usize = 140;

/// 取字串尾端最多 max_chars 個字元（按 char 切、CJK 安全，不會壞字）。純函式。
pub fn tail_chars(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars { return s.to_string(); }
    chars[chars.len() - max_chars..].iter().collect()
}

/// 組 whisper prompt：靜態 steering（繁/簡）+ 近期已定稿文字（條件化解碼）。純函式。
pub fn build_prompt(steering: &str, recent: &str) -> String {
    match (steering.is_empty(), recent.is_empty()) {
        (true, true) => String::new(),
        (false, true) => steering.to_string(),
        (true, false) => recent.to_string(),
        (false, false) => format!("{steering} {recent}"),
    }
}

#[derive(Debug, Clone)]
pub struct WordTs { pub word: String, pub start_sec: f64, pub end_sec: f64 }

/// 一段 whisper verbose_json segment（視窗相對秒）。
#[derive(Debug, Clone)]
pub struct StreamSeg {
    pub start_sec: f64,
    pub end_sec: f64,
    pub text: String,
    pub no_speech_prob: f64,
    pub words: Vec<WordTs>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeldInterim { pub text: String, pub start_sec: f64, pub end_sec: f64 }

pub struct StepInput<'a> {
    pub segs: &'a [StreamSeg],
    pub tail_speech: bool,
    pub silence_boundary: bool,
    pub offset_sec: f64,
    pub captured_sec: f64,
    pub frames_len: usize,
    pub rate: f64,
    pub last_interim_text: &'a str,
    pub agreement_count: u32,
    pub held_interim: Option<HeldInterim>,
    pub session_id: &'a str,
    pub lang: &'a str,
}

pub struct StepOutcome {
    pub finals: Vec<Cue>,
    pub interim: Option<Cue>, // Some(空 source_text)=清除；Some(非空)=顯示；None=不動
    pub new_offset_sec: f64,
    pub trim_to_sample: usize,
    pub new_last_interim: String,
    pub new_agreement_count: u32,
    pub new_held: Option<HeldInterim>,
}

/// 收一段成 final（過 collapse→boilerplate→no_speech）；無論收不收，推 offset 到該段絕對 end（單調）。
fn commit_seg(seg: &StreamSeg, base_offset: f64, session_id: &str, lang: &str,
              finals: &mut Vec<Cue>, new_offset: &mut f64) {
    let abs_start = base_offset + seg.start_sec;
    let abs_end = base_offset + seg.end_sec;
    let t = collapse_repetition(seg.text.trim());
    if !t.is_empty() && seg.no_speech_prob <= NO_SPEECH_COMMIT && !hallucination::is_boilerplate(&t) {
        finals.push(Cue {
            id: derive_cue_id((abs_start * 1000.0).round() as i64),
            session_id: session_id.to_string(),
            start_sec: abs_start, end_sec: abs_end,
            source_text: t, lang: Some(lang.to_string()), status: "final".into(),
        });
    }
    if abs_end > *new_offset { *new_offset = abs_end; }
}

fn empty_interim(session_id: &str) -> Cue {
    Cue { id: format!("{session_id}:interim"), session_id: session_id.to_string(),
          start_sec: 0.0, end_sec: 0.0, source_text: String::new(), lang: None, status: "interim".into() }
}

/// 串流定稿決策（四觸發；純函式）。spec §2.3。
pub fn step(inp: StepInput) -> StepOutcome {
    let mut finals: Vec<Cue> = Vec::new();
    let mut new_offset = inp.offset_sec;
    let mut new_held = inp.held_interim; // 移動而非 clone（inp.held_interim 之後不再讀；partial move 安全）
    let mut new_last_interim = inp.last_interim_text.to_string();
    let mut new_agreement = inp.agreement_count;
    let mut interim: Option<Cue> = None;

    // 空 segs 守衛：原樣不動
    if !inp.segs.is_empty() {
        let last = inp.segs.len() - 1;
        if inp.silence_boundary {
            // T3：commit 全部段（utterance 結束）
            for seg in inp.segs {
                commit_seg(seg, inp.offset_sec, inp.session_id, inp.lang, &mut finals, &mut new_offset);
            }
            interim = Some(empty_interim(inp.session_id));
            new_held = None;
            new_last_interim = String::new();
            new_agreement = 0;
        } else {
            // 路徑 A：T1 先收 segs[:-1]
            if inp.segs.len() > 1 {
                for seg in &inp.segs[..last] {
                    commit_seg(seg, inp.offset_sec, inp.session_id, inp.lang, &mut finals, &mut new_offset);
                }
            }
            let last_seg = &inp.segs[last];
            let last_collapsed = collapse_repetition(last_seg.text.trim());
            new_agreement = if last_collapsed == inp.last_interim_text { inp.agreement_count + 1 } else { 0 };
            new_last_interim = last_collapsed.clone();

            let cap_exceeded = (inp.captured_sec - inp.offset_sec) >= FORCE_COMMIT_SEC;
            if new_agreement >= AGREEMENT_THRESH || cap_exceeded {
                // T2 / T4：收末段
                commit_seg(last_seg, inp.offset_sec, inp.session_id, inp.lang, &mut finals, &mut new_offset);
                interim = Some(empty_interim(inp.session_id));
                new_held = None;
                new_last_interim = String::new();
                new_agreement = 0;
            } else {
                // 留 interim
                let abs_start = inp.offset_sec + last_seg.start_sec;
                let abs_end = inp.offset_sec + last_seg.end_sec;
                interim = Some(Cue {
                    id: format!("{}:interim", inp.session_id),
                    session_id: inp.session_id.to_string(),
                    start_sec: abs_start, end_sec: abs_end,
                    source_text: last_collapsed.clone(),
                    lang: Some(inp.lang.to_string()), status: "interim".into(),
                });
                new_held = Some(HeldInterim { text: last_collapsed, start_sec: abs_start, end_sec: abs_end });
            }
        }
    }

    let trim = (((new_offset - inp.offset_sec) * inp.rate).round() as i64).max(0) as usize;
    let trim_to_sample = trim.min(inp.frames_len);

    StepOutcome { finals, interim, new_offset_sec: new_offset, trim_to_sample,
                  new_last_interim, new_agreement_count: new_agreement, new_held }
}

/// 從 current_start trim 到 target_start 需丟的前端樣本數（夾 0、夾長度）。純函式。
pub fn samples_to_drop(current_start: f64, target_start: f64, rate: f64, len: usize) -> usize {
    if target_start <= current_start { return 0; }
    (((target_start - current_start) * rate).round() as usize).min(len)
}

/// 靈敏度滑桿 (0–1) → 能量 gate RMS 門檻。預設 0.5→0.01（＝原 ENERGY_RMS_THRESH）。純函式。
pub fn vad_threshold_to_rms(threshold: f64) -> f32 {
    (threshold * 0.02) as f32
}

/// 斷句靜音門檻 (ms) → 靜音收句窗(秒)。下限 0.05 為 footgun guard（0 → tail n=0 → 永不轉寫）。純函式。
pub fn min_silence_to_window(ms: i64) -> f64 {
    (ms as f64 / 1000.0).max(0.05)
}

pub struct LoopbackParams {
    pub device_id: Option<String>,
    pub model: String,
    pub source_lang: String, // §4.7 必為 concrete ISO（前端保證）
    pub prompt: String,          // 靜態繁/簡 steering（§3.1；langToWhisper 的 p，空字串=無）
    pub vad_threshold: f64,    // 靈敏度滑桿 → RMS 門檻（映射）
    pub vad_min_silence_ms: i64, // 斷句靜音滑桿 → 靜音收句窗（映射，含 footgun 下限）
}

/// 啟動 loopback 串流 session（arm）：互斥（走 Manager.task）、起擷取執行緒、spawn run_loop（drain-only 初始）。
/// 取 AudioSource（由 arm_audio_source 傳入）；舊 start_loopback_transcription 傳 AudioSource::System。
pub async fn start(
    app: AppHandle,
    mgr: Arc<tokio::sync::Mutex<session::Manager>>,
    source: crate::capture::source::AudioSource,
    downloading: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) -> Result<(), String> {
    let data = crate::data_dir(&app)?.join("subs");
    let (stop, thread, rx) = loopback::start_capture(source)?;
    let app2 = app.clone();
    let mgr2 = mgr.clone();
    {
        let mut m = mgr.lock().await;
        m.stop_task_pub(); // abort 任何進行中 session（含停舊 capture thread）→ 互斥
        m.bump_counter();
        let session_id = m.session_id_str();
        m.reset_cancel();
        m.set_data_dir(data.clone());
        m.set_capture(stop.clone(), thread);
        let cancel = m.cancel_arc();
        let transcribe = m.transcribe_flag();
        let params_h = m.params_handle();
        app.emit("sub-session-reset", SessionResetEvent { session_id: session_id.clone(), no_clock: true }).ok();
        let handle = tauri::async_runtime::spawn(async move {
            if let Err(e) = run_loop(app2.clone(), mgr2, session_id, transcribe, params_h, data, downloading, cancel, rx, stop).await {
                app2.emit("sub-progress", ProgressEvent { phase: "error".into(), done: 0, total: None, message: e }).ok();
            }
        });
        m.set_task(handle);
    }
    Ok(())
}

const CADENCE_MS: u64 = 100;

#[allow(clippy::too_many_arguments)]
async fn run_loop(
    app: AppHandle,
    mgr: Arc<tokio::sync::Mutex<session::Manager>>,
    session_id: String,
    transcribe: Arc<AtomicBool>,
    params_h: Arc<std::sync::Mutex<Option<session::TranscribeParams>>>,
    data: std::path::PathBuf,
    downloading: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    cancel: Arc<AtomicBool>,
    rx: std::sync::mpsc::Receiver<Vec<f32>>,
    _stop: Arc<AtomicBool>,
) -> Result<(), String> {
    // 串流 server 維持 --vad OFF：內建 --vad 在 silero 判 0-speech 的窗會崩潰退出（實測一開即中），
    // 重啟＝60s 模型重載、不可行。要對齊 WhisperLive vad_filter 須走 client-side silero 預過濾（另做）。
    let vad = session::VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: false };
    let rate = loopback::TARGET_RATE as f64;
    const MAX_DRAIN_SEC: f64 = 2.0;

    let mut frames: Vec<f32> = Vec::new();
    let mut offset_sec: f64 = 0.0;
    let mut captured_sec: f64 = 0.0;
    let mut last_speech_sec: f64 = 0.0;
    let mut last_interim_text = String::new();
    let mut agreement_count: u32 = 0;
    let mut held: Option<HeldInterim> = None;
    let mut recent_text = String::new(); // 滾動近期已定稿文字（prompt 上下文，per-session 起空）
    let mut server_ready = false;
    let mut port = 0u16;
    let mut active: Option<(String /*lang*/, String /*prompt*/, String /*model*/)> = None;
    let mut last_level = std::time::Instant::now();
    app.emit("sub-progress", ProgressEvent { phase: "decode".into(), done: 0, total: None, message: "擷取中".into() }).ok();

    while !cancel.load(Ordering::SeqCst) {
        let mut got = 0usize;
        while let Ok(chunk) = rx.try_recv() { got += chunk.len(); frames.extend(chunk); }
        captured_sec += got as f64 / rate;

        // 聲源視覺化：每 ~80ms emit 近窗 RMS（drain 與轉寫期間都送，成本極低）
        if last_level.elapsed() >= std::time::Duration::from_millis(80) {
            let n = (rate * 0.1) as usize; // 近 100ms 窗
            let tail = if frames.len() > n { &frames[frames.len() - n..] } else { &frames[..] };
            let rms = loopback::rms(tail);
            app.emit("capture-level", rms).ok();
            last_level = std::time::Instant::now();
        }

        if !transcribe.load(Ordering::SeqCst) {
            // CC 關 → drain-only 模式：重置 server 狀態、bounded drain（最多保留 MAX_DRAIN_SEC）
            server_ready = false;
            active = None;
            let keep = (MAX_DRAIN_SEC * rate) as usize;
            if frames.len() > keep {
                frames.drain(0..frames.len() - keep);
                offset_sec = captured_sec - MAX_DRAIN_SEC;
            }
            tokio::time::sleep(std::time::Duration::from_millis(CADENCE_MS)).await;
            continue;
        }

        // CC 開：確保 server 就緒（lazy，首次或 CC 重開時才載模型）
        if !server_ready {
            let p_opt = { params_h.lock().unwrap().clone() }; // guard dropped before any await
            let p = match p_opt {
                Some(p) => p,
                None => {
                    tokio::time::sleep(std::time::Duration::from_millis(CADENCE_MS)).await;
                    continue;
                }
            };
            port = session::ensure_server(&app, &mgr, &data, &cancel, &p.model, &vad, &downloading).await?;
            active = Some((p.source_lang.clone(), p.prompt.clone(), p.model.clone()));
            server_ready = true;
            // reset offset/buffer 邊界，避免把 drain-only 期間的舊音當待轉
            offset_sec = captured_sec;
            frames.clear();
            recent_text.clear();
        }

        let http = { mgr.lock().await.http_clone().ok_or("no http")? };
        let (lang, prompt, model) = active.as_ref().unwrap().clone();
        // 從 params_h 讀最新的 vad 參數（每輪重讀以支援動態調整）
        let (rms_thresh, silence_window) = {
            let lock = params_h.lock().unwrap();
            match lock.as_ref() {
                Some(p) => (vad_threshold_to_rms(p.vad_threshold), min_silence_to_window(p.vad_min_silence_ms)),
                None => (vad_threshold_to_rms(0.5), min_silence_to_window(100)),
            }
        };

        let buffer_len = frames.len() as f64 / rate;
        let tail_speech = buffer_len >= MIN_SPEECH_BUFFER_SEC
            && loopback::tail_has_speech(&frames, silence_window, rms_thresh);
        if tail_speech { last_speech_sec = captured_sec; }
        let trailing_silence = captured_sec - last_speech_sec;
        let silence_boundary = trailing_silence >= FINALIZE_SILENCE_SEC && held.is_some();

        // 靜態 steering + 近期已定稿文字 → 條件化解碼（同音字/專有名詞一致性）。
        let combined_prompt = build_prompt(&prompt, &recent_text);
        let pr = if combined_prompt.is_empty() { None } else { Some(combined_prompt.as_str()) };

        if tail_speech || silence_boundary {
            // 路徑 A / 路徑 B：轉寫 + step
            let segs = match whisper::transcribe_bytes(&http, port, &frames, &lang, pr).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[stream] transcribe 失敗（server 可能崩潰），重啟續跑：{e}");
                    mgr.lock().await.invalidate_server();
                    port = session::ensure_server(&app, &mgr, &data, &cancel, &model, &vad, &downloading).await?;
                    continue;
                }
            };
            let o = step(StepInput {
                segs: &segs, tail_speech, silence_boundary,
                offset_sec, captured_sec, frames_len: frames.len(), rate,
                last_interim_text: &last_interim_text, agreement_count,
                held_interim: held.clone(), session_id: &session_id, lang: &lang,
            });
            for f in &o.finals {
                app.emit("sub-cue", f).ok();
                recent_text.push_str(&f.source_text); // 累積已定稿（已過 collapse，不會灌重複牆）
            }
            recent_text = tail_chars(&recent_text, RECENT_CONTEXT_CHARS);
            if let Some(it) = &o.interim { app.emit("sub-cue", it).ok(); }
            if o.trim_to_sample > 0 { frames.drain(0..o.trim_to_sample); }
            offset_sec = o.new_offset_sec;
            last_interim_text = o.new_last_interim;
            agreement_count = o.new_agreement_count;
            held = o.new_held;
        } else if held.is_none() {
            // 路徑 C：純靜音 → 推 offset 過前導靜音、留 SILENCE_KEEP_SEC 尾段（不清已定稿）
            let target = (captured_sec - SILENCE_KEEP_SEC).max(offset_sec);
            let drop = samples_to_drop(offset_sec, target, rate, frames.len());
            if drop > 0 { frames.drain(0..drop); offset_sec = target; }
        }
        // else：line_active 但 trailing_silence < FINALIZE_SILENCE_SEC → 純等待（不轉寫、不 trim，避免微停頓誤切）

        tokio::time::sleep(std::time::Duration::from_millis(CADENCE_MS)).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vad_threshold_maps_to_rms() {
        assert!((vad_threshold_to_rms(0.5) - 0.01).abs() < 1e-6);
        assert!((vad_threshold_to_rms(1.0) - 0.02).abs() < 1e-6);
        assert_eq!(vad_threshold_to_rms(0.0), 0.0);
    }

    #[test]
    fn min_silence_maps_with_floor() {
        assert!((min_silence_to_window(100) - 0.1).abs() < 1e-9);
        assert!((min_silence_to_window(700) - 0.7).abs() < 1e-9);
        assert!((min_silence_to_window(0) - 0.05).abs() < 1e-9);   // footgun guard
        assert!((min_silence_to_window(30) - 0.05).abs() < 1e-9);  // < floor → floor
    }

    #[test]
    fn samples_to_drop_basic() {
        assert_eq!(samples_to_drop(9.0, 10.0, 16000.0, 48000), 16000);
        assert_eq!(samples_to_drop(10.0, 9.0, 16000.0, 48000), 0);
        assert_eq!(samples_to_drop(0.0, 100.0, 16000.0, 800), 800);
    }

    // ── Task 2: step() 型別 ──────────────────────────────────────────────────
    fn sseg(s: f64, e: f64, t: &str, nsp: f64) -> StreamSeg {
        StreamSeg { start_sec: s, end_sec: e, text: t.into(), no_speech_prob: nsp, words: vec![] }
    }
    fn base_input<'a>(segs: &'a [StreamSeg], tail_speech: bool, silence_boundary: bool,
                      offset: f64, captured: f64, last_interim: &'a str, agree: u32) -> StepInput<'a> {
        StepInput {
            segs, tail_speech, silence_boundary, offset_sec: offset, captured_sec: captured,
            frames_len: ((captured - offset) * 16000.0) as usize, rate: 16000.0,
            last_interim_text: last_interim, agreement_count: agree,
            held_interim: None, session_id: "s", lang: "zh",
        }
    }

    #[test]
    fn step_empty_segs_noop() {
        let segs: Vec<StreamSeg> = vec![];
        let o = step(base_input(&segs, true, false, 5.0, 6.0, "前", 2));
        assert!(o.finals.is_empty());
        assert!(o.interim.is_none());
        assert_eq!(o.new_offset_sec, 5.0);
        assert_eq!(o.trim_to_sample, 0);
        assert_eq!(o.new_agreement_count, 2);
        assert_eq!(o.new_last_interim, "前");
    }

    #[test]
    fn step_t1_commits_all_but_last() {
        // 兩段：第一段定稿（絕對 id = (5.0)*1000=5000）、末段留 interim
        let segs = [sseg(0.0, 1.0, "第一句", 0.1), sseg(1.0, 2.0, "第二句進行", 0.1)];
        let o = step(base_input(&segs, true, false, 5.0, 7.0, "", 0));
        assert_eq!(o.finals.len(), 1);
        assert_eq!(o.finals[0].id, "5000");       // 絕對 startMs = (5.0+0.0)*1000
        assert_eq!(o.finals[0].source_text, "第一句");
        assert_eq!(o.finals[0].status, "final");
        assert_eq!(o.new_offset_sec, 6.0);         // 推到第一段絕對 end = 5.0+1.0
        assert_eq!(o.trim_to_sample, 16000);       // (6.0-5.0)*16000
        let it = o.interim.unwrap();
        assert_eq!(it.id, "s:interim");
        assert_eq!(it.source_text, "第二句進行");
        assert_eq!(it.status, "interim");
        assert!(o.new_held.is_some());
    }

    #[test]
    fn step_t2_agreement_commits_last() {
        // 單段、collapsed 文字與 last_interim 相同、agreement+1 達門檻 → 收末段 + 空 interim
        let segs = [sseg(0.0, 1.5, "穩定的一句", 0.1)];
        let o = step(base_input(&segs, true, false, 5.0, 6.5, "穩定的一句", AGREEMENT_THRESH - 1));
        assert_eq!(o.new_agreement_count, 0);      // commit 後 reset
        assert_eq!(o.finals.len(), 1);
        assert_eq!(o.finals[0].source_text, "穩定的一句");
        assert_eq!(o.interim.as_ref().unwrap().source_text, ""); // 空 interim 清除
        assert!(o.new_held.is_none());
        assert_eq!(o.new_offset_sec, 6.5);
    }

    #[test]
    fn step_t2_agreement_breaks_repetition_wall_via_collapse() {
        // 成長重複牆：raw 每輪變長，但 collapse 後穩定 → agreement 比對/儲存皆用 collapsed → 抓得到。
        let segs = [sseg(0.0, 3.0, "賣掉賣掉賣掉賣掉賣掉", 0.1)];
        // (A) 未達門檻：collapsed-agreement 累計、last_interim 存 collapsed、僅 interim（未 commit）
        let a = step(base_input(&segs, true, false, 5.0, 8.0, "賣掉", 0));
        assert_eq!(a.new_agreement_count, 1);
        assert_eq!(a.new_last_interim, "賣掉");                       // 比對/儲存皆用 collapsed
        assert!(a.finals.is_empty());
        assert_eq!(a.interim.as_ref().unwrap().source_text, "賣掉");   // interim 也收摺顯示
        // (B) 達門檻：commit 收摺後文字、reset last_interim/agreement
        let b = step(base_input(&segs, true, false, 5.0, 8.0, "賣掉", AGREEMENT_THRESH - 1));
        assert_eq!(b.finals.len(), 1);
        assert_eq!(b.finals[0].source_text, "賣掉");
        assert_eq!(b.new_last_interim, "");                          // commit 後 reset（§8 不變式 4）
        assert_eq!(b.new_agreement_count, 0);
    }

    #[test]
    fn step_t4_force_commit_on_cap() {
        // 未達 agreement 但窗 >= FORCE_COMMIT_SEC → 強制收末段
        let segs = [sseg(0.0, 31.0, "很長一段沒停頓", 0.1)];
        let o = step(base_input(&segs, true, false, 0.0, 31.0, "別的", 0));
        assert_eq!(o.finals.len(), 1);
        assert_eq!(o.interim.as_ref().unwrap().source_text, "");
    }

    #[test]
    fn step_t3_silence_commits_all() {
        let segs = [sseg(0.0, 1.0, "前句", 0.1), sseg(1.0, 2.2, "後句", 0.1)];
        let o = step(base_input(&segs, false, true, 5.0, 8.0, "", 0));
        assert_eq!(o.finals.len(), 2);             // 全收
        assert_eq!(o.new_offset_sec, 7.2);         // 最後 seg 絕對 end = 5.0+2.2
        assert_eq!(o.interim.as_ref().unwrap().source_text, "");
        assert!(o.new_held.is_none());
    }

    #[test]
    fn step_no_speech_and_boilerplate_skip_but_advance() {
        // 高 no_speech 段：不收文字、offset 仍推進
        let segs = [sseg(0.0, 1.0, "雜訊", 0.9), sseg(1.0, 2.0, "真句進行", 0.1)];
        let o = step(base_input(&segs, true, false, 0.0, 2.0, "", 0));
        assert!(o.finals.is_empty());              // 第一段被 no_speech gate 擋、第二段是 interim
        assert_eq!(o.new_offset_sec, 1.0);         // 仍推進過第一段
        // boilerplate
        let segs2 = [sseg(0.0, 1.0, "字幕志愿者 李宗盛", 0.1), sseg(1.0, 2.0, "真句", 0.1)];
        let o2 = step(base_input(&segs2, true, false, 0.0, 2.0, "", 0));
        assert!(o2.finals.is_empty());
        assert_eq!(o2.new_offset_sec, 1.0);
    }

    // ── 回灌 prompt 上下文 ────────────────────────────────────────────────────
    #[test]
    fn tail_chars_keeps_last_n_cjk_safe() {
        assert_eq!(tail_chars("一二三四五", 3), "三四五");
        assert_eq!(tail_chars("一二三", 5), "一二三"); // 短於上限原樣
        assert_eq!(tail_chars("", 3), "");
        assert_eq!(tail_chars("abcde", 2), "de");
    }

    #[test]
    fn build_prompt_combines_steering_and_recent() {
        assert_eq!(build_prompt("", ""), "");
        assert_eq!(build_prompt("以下是繁體", ""), "以下是繁體");
        assert_eq!(build_prompt("", "前文內容"), "前文內容");
        assert_eq!(build_prompt("以下是繁體", "前文內容"), "以下是繁體 前文內容");
    }

    // ── Task 1: collapse_repetition ──────────────────────────────────────────
    #[test]
    fn collapse_repetition_cases() {
        // 單字疊字：最小週期 p0=1 < MIN_UNIT(2) → 永不收（與次數無關）
        assert_eq!(collapse_repetition("哈哈哈哈哈哈"), "哈哈哈哈哈哈");
        assert_eq!(collapse_repetition("好好好"), "好好好");
        // ≥2 字幻覺牆 → 收成 1 份
        assert_eq!(collapse_repetition("賣掉賣掉賣掉賣掉賣掉賣掉"), "賣掉"); // ×6
        assert_eq!(collapse_repetition("應該會賣掉應該會賣掉應該會賣掉"), "應該會賣掉"); // 5字×3
        assert_eq!(collapse_repetition("123123123"), "123"); // 3字×3
        // 疊字開頭的牆（最廣覆蓋修盲點：p=1 只覆蓋 2、p=4/3 覆蓋全長 → 收）
        assert_eq!(collapse_repetition("謝謝大家謝謝大家謝謝大家"), "謝謝大家"); // 4字×3
        assert_eq!(collapse_repetition("媽媽說媽媽說媽媽說"), "媽媽說");       // 3字×3
        // ≥2字×2（未達 MIN_REPEATS=3）→ 不收
        assert_eq!(collapse_repetition("ABAB"), "ABAB");
        // ≥2字×3 → 收（釘決定性邊界）
        assert_eq!(collapse_repetition("ABABAB"), "AB");
        // 混合單字 run（各自 p0=1）→ 不收
        assert_eq!(collapse_repetition("甲甲甲乙乙乙乙"), "甲甲甲乙乙乙乙");
        // 邊界
        assert_eq!(collapse_repetition(""), "");
        assert_eq!(collapse_repetition("你好"), "你好");
        // 牆夾在正常文字中
        assert_eq!(collapse_repetition("然後賣掉賣掉賣掉賣掉好嗎"), "然後賣掉好嗎");
    }
}
