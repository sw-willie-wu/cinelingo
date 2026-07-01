use std::path::{Path, PathBuf};
use super::cue::{derive_cue_id, Cue};
use super::plan;

/// 與前端 normKey 等價：小寫化 + 統一分隔符。Windows 檔案系統大小寫不敏感 → 同一支影片每次映射同鍵。
pub fn normalize(video_path: &str) -> String {
    video_path.to_lowercase().replace('\\', "/")
}

/// SHA-256 hex（重用 P3 既有 sha2；同 download.rs 的 finalize/{:x} 形式）。
fn hash_path(video_path: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(normalize(video_path).as_bytes());
    format!("{:x}", h.finalize())
}

/// 回快取 SRT 路徑：`<app_data_dir>/whisper-subs/<hash(normalize(video))>/<cache_key_lang>.whisper.srt`。
/// JSON 路徑由呼叫端以 `.with_extension("json")` 取得（".whisper.srt" → ".whisper.json"）。
/// 注意：根目錄是 app_data_dir 本身，不是 `<app_data>/subs/`。
pub fn cache_path_for(app_data_dir: &Path, video_path: &str, cache_key_lang: &str) -> PathBuf {
    app_data_dir
        .join("whisper-subs")
        .join(hash_path(video_path))
        .join(format!("{cache_key_lang}.whisper.srt"))
}

/// 秒 → "HH:MM:SS,mmm"，以 round(sec*1000) ms 計（與 derive_cue_id 同一捨入 → round-trip 穩定）。
fn fmt_ts(sec: f64) -> String {
    let ms = (sec * 1000.0).round().max(0.0) as i64;
    let h = ms / 3_600_000;
    let m = (ms % 3_600_000) / 60_000;
    let s = (ms % 60_000) / 1000;
    let milli = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, milli)
}

/// cues → 標準 SRT（序號、時間軸、文字、空行），依 start 升冪。
pub fn cues_to_srt(cues: &[Cue]) -> String {
    let mut sorted: Vec<&Cue> = cues.iter().collect();
    sorted.sort_by(|a, b| a.start_sec.total_cmp(&b.start_sec));
    let mut out = String::new();
    for (i, c) in sorted.iter().enumerate() {
        out.push_str(&format!("{}\n", i + 1));
        out.push_str(&format!("{} --> {}\n", fmt_ts(c.start_sec), fmt_ts(c.end_sec)));
        out.push_str(&c.source_text);
        out.push_str("\n\n");
    }
    out
}

/// "HH:MM:SS,mmm"（或 '.' 毫秒分隔）→ 秒。失敗回 None。
fn parse_ts(s: &str) -> Option<f64> {
    let (hms, ms) = s.split_once(',').or_else(|| s.split_once('.'))?;
    let parts: Vec<&str> = hms.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let h: f64 = parts[0].trim().parse().ok()?;
    let m: f64 = parts[1].trim().parse().ok()?;
    let sec: f64 = parts[2].trim().parse().ok()?;
    let milli: f64 = ms.trim().parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + sec + milli / 1000.0)
}

fn parse_ts_line(line: &str) -> Option<(f64, f64)> {
    let (a, b) = line.split_once("-->")?;
    Some((parse_ts(a.trim())?, parse_ts(b.trim())?))
}

/// 解析快取 SRT；cue id 用 startMs 規則（與 merge.rs / 前端 deriveCueId 一致）→ seed 能與 live 去重。
/// session_id 留空、lang None（呼叫端蓋上當前 sessionId）。
pub fn parse_srt_cached(text: &str) -> Vec<Cue> {
    let mut out = Vec::new();
    let norm = text.replace("\r\n", "\n");
    for block in norm.split("\n\n") {
        let lines: Vec<&str> = block.lines().filter(|l| !l.trim().is_empty()).collect();
        let Some(ti) = lines.iter().position(|l| l.contains("-->")) else { continue };
        let Some((start, end)) = parse_ts_line(lines[ti]) else { continue };
        let body = lines[ti + 1..].join("\n").trim().to_string();
        if body.is_empty() {
            continue;
        }
        out.push(Cue {
            id: derive_cue_id((start * 1000.0).round() as i64),
            session_id: String::new(),
            start_sec: start,
            end_sec: end,
            source_text: body,
            lang: None,
            status: "final".into(),
            ..Default::default()
        });
    }
    out
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CacheMeta {
    pub coverage: Vec<(f64, f64)>,
    pub video_path: String,
    pub duration_sec: f64,
    pub lang: Option<String>,
    // 轉寫參數標記（A3）：舊 json 缺欄 → serde 自動填 None（向後相容）。
    pub model: Option<String>,
    pub vad_threshold: Option<f64>,
    pub vad_min_silence_ms: Option<i64>,
}

/// 寫快取時要標記的轉寫參數（供 cache_params_match 比對）。
pub struct CacheParams<'a> {
    pub model: &'a str,
    pub vad_threshold: f64,
    pub vad_min_silence_ms: i64,
}

/// 讀快取 cues；每條蓋上傳入 sessionId（否則前端依 sessionId 過濾會丟掉）。缺檔/讀失敗 → 空。
pub fn read_cached_cues(srt_path: &Path, session_id: &str) -> Vec<Cue> {
    match std::fs::read_to_string(srt_path) {
        Ok(text) => {
            let mut cues = parse_srt_cached(&text);
            for c in &mut cues {
                c.session_id = session_id.to_string();
            }
            cues
        }
        Err(_) => Vec::new(),
    }
}

/// 讀整份快取 meta（coverage + 參數標記）。缺檔/解析失敗 → None。
/// （coverage 由呼叫端取 `m.coverage`；session 端讀一次 meta 同時做 stale 判定與 coverage seed。）
pub fn read_cached_meta(json_path: &Path) -> Option<CacheMeta> {
    let text = std::fs::read_to_string(json_path).ok()?;
    serde_json::from_str::<CacheMeta>(&text).ok()
}

/// 快取 meta 的轉寫參數是否與目前相符（model + VAD）。任一為 None（舊快取）→ 視為不符。
/// threshold 用 epsilon 1e-4（與 `session::vad_eq` 同慣例），minSilenceMs/model 精確比對。
pub fn cache_params_match(meta: &CacheMeta, params: &CacheParams) -> bool {
    meta.model.as_deref() == Some(params.model)
        && meta.vad_min_silence_ms == Some(params.vad_min_silence_ms)
        && meta.vad_threshold.is_some_and(|t| (t - params.vad_threshold).abs() < 1e-4)
}

/// 覆寫 srt（全 cues）+ json（coverage + meta，含轉寫參數標記）。建目錄。
#[allow(clippy::too_many_arguments)]
pub fn write_cache(
    srt_path: &Path,
    json_path: &Path,
    cues: &[Cue],
    coverage: &plan::Intervals,
    video_path: &str,
    duration_sec: f64,
    lang: Option<&str>,
    params: &CacheParams,
) -> std::io::Result<()> {
    if let Some(dir) = srt_path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(srt_path, cues_to_srt(cues))?;
    let meta = CacheMeta {
        coverage: coverage.clone(),
        video_path: video_path.to_string(),
        duration_sec,
        lang: lang.map(|s| s.to_string()),
        model: Some(params.model.to_string()),
        vad_threshold: Some(params.vad_threshold),
        vad_min_silence_ms: Some(params.vad_min_silence_ms),
    };
    let bytes = serde_json::to_vec_pretty(&meta).unwrap_or_default();
    std::fs::write(json_path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use super::super::cue::{derive_cue_id, Cue};

    #[test]
    fn cache_path_stable_and_normalized() {
        let root = Path::new("C:/data");
        let a = cache_path_for(root, "C:\\Movies\\Film.MKV", "ja");
        let b = cache_path_for(root, "c:/movies/film.mkv", "ja");
        assert_eq!(a, b); // 大小寫 + 分隔符正規化 → 同路徑
    }

    #[test]
    fn cache_path_lang_splits_trad_simp() {
        let root = Path::new("C:/data");
        let trad = cache_path_for(root, "C:/v.mkv", "zh-Hant");
        let simp = cache_path_for(root, "C:/v.mkv", "zh-Hans");
        assert_ne!(trad, simp);
        assert!(trad.to_string_lossy().ends_with("zh-Hant.whisper.srt"));
        assert!(simp.to_string_lossy().ends_with("zh-Hans.whisper.srt"));
    }

    #[test]
    fn cache_path_auto_literal_under_whisper_subs() {
        let p = cache_path_for(Path::new("C:/data"), "C:/v.mkv", "auto");
        let s = p.to_string_lossy();
        assert!(s.ends_with("auto.whisper.srt"));
        assert!(s.contains("whisper-subs"));
    }

    #[test]
    fn cues_to_srt_formats_sorted() {
        let cues = vec![
            Cue { id: "x".into(), session_id: "".into(), start_sec: 60.0, end_sec: 61.5,
                  source_text: "second".into(), lang: None, status: "final".into(), ..Default::default() },
            Cue { id: "y".into(), session_id: "".into(), start_sec: 1.0, end_sec: 2.0,
                  source_text: "first".into(), lang: None, status: "final".into(), ..Default::default() },
        ];
        let srt = cues_to_srt(&cues);
        assert!(srt.starts_with("1\n00:00:01,000 --> 00:00:02,000\nfirst\n\n"));
        assert!(srt.contains("2\n00:01:00,000 --> 00:01:01,500\nsecond"));
    }

    #[test]
    fn parse_srt_cached_uses_start_ms_id() {
        let srt = "1\n00:00:10,500 --> 00:00:12,000\nhi\n\n";
        let cues = parse_srt_cached(srt);
        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].id, derive_cue_id(10500));
        assert_eq!(cues[0].start_sec, 10.5);
        assert_eq!(cues[0].source_text, "hi");
        assert_eq!(cues[0].session_id, ""); // 解析出 session 空，蓋 sessionId 由呼叫端做
    }

    #[test]
    fn srt_round_trip_preserves_ids() {
        let cues = vec![
            Cue { id: derive_cue_id(10500), session_id: "s1".into(), start_sec: 10.5, end_sec: 12.0,
                  source_text: "hi".into(), lang: Some("ja".into()), status: "final".into(), ..Default::default() },
            Cue { id: derive_cue_id(60000), session_id: "s1".into(), start_sec: 60.0, end_sec: 62.0,
                  source_text: "bye\nnow".into(), lang: None, status: "final".into(), ..Default::default() },
        ];
        let parsed = parse_srt_cached(&cues_to_srt(&cues));
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].id, cues[0].id);
        assert_eq!(parsed[1].id, cues[1].id);
        assert_eq!(parsed[0].id, derive_cue_id((10.5_f64 * 1000.0).round() as i64));
        assert_eq!(parsed[1].source_text, "bye\nnow");
    }

    #[test]
    fn cache_meta_serde_round_trip() {
        let meta = CacheMeta {
            coverage: vec![(0.0, 30.0), (60.0, 90.0)],
            video_path: "C:/v.mkv".into(),
            duration_sec: 120.0,
            lang: Some("ja".into()),
            model: Some("turbo".into()),
            vad_threshold: Some(0.5),
            vad_min_silence_ms: Some(100),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let back: CacheMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coverage, vec![(0.0, 30.0), (60.0, 90.0)]);
        assert_eq!(back.duration_sec, 120.0);
        assert_eq!(back.lang.as_deref(), Some("ja"));
        assert_eq!(back.model.as_deref(), Some("turbo"));
        assert_eq!(back.vad_threshold, Some(0.5));
        assert_eq!(back.vad_min_silence_ms, Some(100));
        // camelCase 鍵確認
        assert!(json.contains("\"vadThreshold\""));
        assert!(json.contains("\"vadMinSilenceMs\""));
    }

    #[test]
    fn cache_meta_back_compat_missing_param_fields() {
        // 舊 json（功能上線前）沒有 model/vadThreshold/vadMinSilenceMs → 反序列化為 None
        let old = r#"{"coverage":[[0.0,30.0]],"videoPath":"C:/v.mkv","durationSec":120.0,"lang":"ja"}"#;
        let m: CacheMeta = serde_json::from_str(old).unwrap();
        assert_eq!(m.coverage, vec![(0.0, 30.0)]);
        assert_eq!(m.lang.as_deref(), Some("ja"));
        assert_eq!(m.model, None);
        assert_eq!(m.vad_threshold, None);
        assert_eq!(m.vad_min_silence_ms, None);
    }

    fn cp(model: &str, t: f64, ms: i64) -> CacheParams<'_> {
        CacheParams { model, vad_threshold: t, vad_min_silence_ms: ms }
    }

    #[test]
    fn cache_params_match_cases() {
        let meta = CacheMeta {
            coverage: vec![],
            video_path: "C:/v.mkv".into(),
            duration_sec: 0.0,
            lang: None,
            model: Some("turbo".into()),
            vad_threshold: Some(0.50),
            vad_min_silence_ms: Some(100),
        };
        // 全相符（threshold 在 epsilon 內）
        assert!(cache_params_match(&meta, &cp("turbo", 0.50, 100)));
        assert!(cache_params_match(&meta, &cp("turbo", 0.50001, 100))); // < 1e-4
        // 各不符
        assert!(!cache_params_match(&meta, &cp("large-v3", 0.50, 100))); // model
        assert!(!cache_params_match(&meta, &cp("turbo", 0.5002, 100))); // threshold 略過界（2e-4 > 1e-4）
        assert!(!cache_params_match(&meta, &cp("turbo", 0.55, 100))); // threshold 遠超
        assert!(!cache_params_match(&meta, &cp("turbo", 0.50, 200))); // minSilence
        // 任一 None（舊快取）→ 不符
        let legacy = CacheMeta { model: None, ..meta.clone() };
        assert!(!cache_params_match(&legacy, &cp("turbo", 0.50, 100)));
    }

    #[test]
    fn write_then_read_cache_round_trip() {
        let dir = std::env::temp_dir().join(format!("lmpv-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let srt = dir.join("ja.whisper.srt");
        let json = dir.join("ja.whisper.json");
        let cues = vec![Cue {
            id: derive_cue_id(1000), session_id: "s1".into(), start_sec: 1.0, end_sec: 2.0,
            source_text: "hi".into(), lang: Some("ja".into()), status: "final".into(),
            ..Default::default()
        }];
        write_cache(
            &srt, &json, &cues, &vec![(0.0, 30.0)], "C:/v.mkv", 120.0, Some("ja"),
            &CacheParams { model: "turbo", vad_threshold: 0.5, vad_min_silence_ms: 100 },
        )
        .unwrap();
        let read_cues = read_cached_cues(&srt, "s2");
        assert_eq!(read_cues.len(), 1);
        assert_eq!(read_cues[0].id, derive_cue_id(1000));
        assert_eq!(read_cues[0].session_id, "s2"); // 蓋上傳入 sessionId
        // meta 取回：coverage + 參數比對
        let meta = read_cached_meta(&json).unwrap();
        assert_eq!(meta.coverage, vec![(0.0, 30.0)]);
        assert!(cache_params_match(&meta, &cp("turbo", 0.5, 100)));
        assert!(!cache_params_match(&meta, &cp("small", 0.5, 100)));
        // 缺檔 → 空 / None
        assert!(read_cached_cues(&dir.join("none.srt"), "s3").is_empty());
        assert!(read_cached_meta(&dir.join("none.json")).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }
}
