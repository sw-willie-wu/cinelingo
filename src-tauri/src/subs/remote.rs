use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CcTrack {
    pub lang: String,
    pub label: String,
    pub auto: bool,
    pub vtt_url: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VideoFormat {
    pub itag: String,
    pub height: u32,
    pub fps: u32,
    pub codec: String,
    pub tbr: f64,
    pub url: String,
}

/// Result of purely picking from yt-dlp `-J` JSON. A `None` url means the JSON had no
/// single-URL format → the caller fills it via a `-g` fallback.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Picked {
    pub playback_url: Option<String>,
    pub audio_url: Option<String>,
    pub http_headers: BTreeMap<String, String>,
    pub duration_sec: f64,
    pub is_live: bool,
    pub cc_tracks: Vec<CcTrack>,
    pub title: Option<String>,
    pub videos: Vec<VideoFormat>,
}

fn is_single_http(f: &serde_json::Value) -> bool {
    let proto = f.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
    (proto == "https" || proto == "http") && f.get("url").and_then(|v| v.as_str()).is_some()
}
fn headers_of(f: &serde_json::Value) -> BTreeMap<String, String> {
    f.get("http_headers")
        .and_then(|v| v.as_object())
        .map(|m| m.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
        .unwrap_or_default()
}

/// Pure pick (no network/spawn). playback = combined (has video+audio); audio = audio-only best; both need single http url.
pub fn pick_resolved(j: &serde_json::Value) -> Picked {
    let is_live = j.get("is_live").and_then(|v| v.as_bool()).unwrap_or(false);
    let duration_sec = j.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let empty = vec![];
    let formats = j.get("formats").and_then(|v| v.as_array()).unwrap_or(&empty);

    let none_str = |f: &serde_json::Value, k: &str| f.get(k).and_then(|v| v.as_str()) == Some("none");
    let tbr = |f: &serde_json::Value| f.get("tbr").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let audio = formats.iter()
        .filter(|f| none_str(f, "vcodec") && !none_str(f, "acodec") && is_single_http(f))
        .max_by(|a, b| tbr(a).total_cmp(&tbr(b)));
    let combined = formats.iter()
        .filter(|f| !none_str(f, "vcodec") && !none_str(f, "acodec") && is_single_http(f))
        .max_by(|a, b| tbr(a).total_cmp(&tbr(b)));

    let url_of = |f: Option<&serde_json::Value>| f.and_then(|x| x.get("url")).and_then(|v| v.as_str()).map(|s| s.to_string());
    let http_headers = combined.map(headers_of).filter(|m| !m.is_empty())
        .or_else(|| audio.map(headers_of))
        .unwrap_or_default();

    let cc_tracks = collect_cc(j);
    let title = j.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());

    // video-only DASH 軌：每 height 取最佳編碼器（AV1 > VP9 > H264）、同編碼器取最高 tbr，降冪。
    let codec_short = |f: &serde_json::Value| f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("")
        .split('.').next().unwrap_or("").to_string();
    let codec_rank = |c: &str| match c { "av01" => 0u8, "vp9" | "vp09" => 1, "avc1" | "h264" => 2, _ => 3 };
    let height_of = |f: &serde_json::Value| f.get("height").and_then(|v| v.as_u64())
        .or_else(|| f.get("width").and_then(|v| v.as_u64()).map(|w| w * 9 / 16))
        .unwrap_or(0) as u32;
    let mut by_height: std::collections::BTreeMap<u32, VideoFormat> = std::collections::BTreeMap::new();
    for f in formats.iter()
        .filter(|f| !none_str(f, "vcodec") && none_str(f, "acodec") && is_single_http(f)) {
        let h = height_of(f);
        if h == 0 { continue; }
        let vf = VideoFormat {
            itag: f.get("format_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            height: h,
            fps: f.get("fps").and_then(|v| v.as_f64()).unwrap_or(0.0).round() as u32,
            codec: codec_short(f),
            tbr: tbr(f),
            url: f.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };
        by_height.entry(h)
            .and_modify(|cur| {
                let better = codec_rank(&vf.codec) < codec_rank(&cur.codec)
                    || (codec_rank(&vf.codec) == codec_rank(&cur.codec) && vf.tbr > cur.tbr);
                if better { *cur = vf.clone(); }
            })
            .or_insert(vf);
    }
    let videos: Vec<VideoFormat> = by_height.into_values().rev().collect();

    Picked {
        playback_url: url_of(combined),
        audio_url: url_of(audio),
        http_headers,
        duration_sec,
        is_live,
        cc_tracks,
        title,
        videos,
    }
}

/// Collect the vtt track per language from subtitles + automatic_captions.
fn collect_cc(j: &serde_json::Value) -> Vec<CcTrack> {
    let mut out = Vec::new();
    for (key, auto) in [("subtitles", false), ("automatic_captions", true)] {
        if let Some(map) = j.get(key).and_then(|v| v.as_object()) {
            for (lang, arr) in map {
                if let Some(vtt) = arr.as_array().and_then(|a| {
                    a.iter().find(|e| e.get("ext").and_then(|v| v.as_str()) == Some("vtt"))
                }) {
                    if let Some(url) = vtt.get("url").and_then(|v| v.as_str()) {
                        out.push(CcTrack {
                            lang: lang.clone(),
                            label: format!("{lang}{}", if auto { "（自動）" } else { "" }),
                            auto,
                            vtt_url: url.to_string(),
                        });
                    }
                }
            }
        }
    }
    out
}

/// 組 ffmpeg「從 remote playback_url 抽音訊→16kHz mono s16 wav」的參數（純函式，可測）。
/// headers 來自 pick_resolved（key 如 "User-Agent"）：UA 大小寫不敏感抽成 -user_agent；
/// 其餘 header 以 "K: V\r\n" 串接給 -headers（ffmpeg 慣例）。空則省略對應旗標。-y 必帶（避免撞檔互動 hang）。
pub fn build_remote_decode_args(
    playback_url: &str,
    headers: &BTreeMap<String, String>,
    dest: &str,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    let ua = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("user-agent")).map(|(_, v)| v.clone());
    let other: String = headers
        .iter()
        .filter(|(k, _)| !k.eq_ignore_ascii_case("user-agent"))
        .map(|(k, v)| format!("{k}: {v}\r\n"))
        .collect();
    if !other.is_empty() {
        args.push("-headers".into());
        args.push(other);
    }
    if let Some(ua) = ua {
        args.push("-user_agent".into());
        args.push(ua);
    }
    args.push("-i".into());
    args.push(playback_url.into());
    for s in ["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav", "-y"] {
        args.push(s.into());
    }
    args.push(dest.into());
    args
}

/// 從 muxed playback_url 抽音訊解碼成 16kHz mono s16 wav（帶 headers，否則 403）。muxed 不限速、整支秒級。
pub async fn decode_remote_audio(
    ffmpeg: &Path,
    playback_url: &str,
    headers: &BTreeMap<String, String>,
    dest: &Path,
) -> Result<PathBuf, String> {
    let out = dest.to_path_buf();
    let dest_str = out.to_string_lossy();
    let args = build_remote_decode_args(playback_url, headers, &dest_str);
    let mut cmd = Command::new(ffmpeg);
    cmd.args(&args).kill_on_drop(true);
    no_window(&mut cmd);
    let st = cmd.status().await.map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(format!("ffmpeg remote decode exit {st}"));
    }
    Ok(out)
}

/// 抽 remote 音訊成 16k wav。
/// 主路徑：`yt-dlp -f ba` 下載完整音訊檔（繞過 YouTube 對直連 URL 的 n 限速，實測 ~58MB/s vs 串流 ~2x）
/// → ffmpeg 解成 wav。退路：舊行為——ffmpeg 直接串流 playback_url（會被限速但通常可完成）；
/// 再失敗 → 用 watch_url 走完整 resolve_remote 刷新 URL+headers 重試一次。
pub async fn decode_remote_with_retry(
    ffmpeg: &Path,
    yt_dlp: &Path,
    watch_url: &str,
    playback_url: &str,
    headers: &BTreeMap<String, String>,
    dest: &Path,
) -> Result<PathBuf, String> {
    match download_audio_then_decode(ffmpeg, yt_dlp, watch_url, dest).await {
        Ok(p) => return Ok(p),
        Err(e) => eprintln!("[subs] yt-dlp 下載抽音失敗，退回串流 URL：{e}"),
    }
    match decode_remote_audio(ffmpeg, playback_url, headers, dest).await {
        Ok(p) => Ok(p),
        Err(first) => {
            let fresh = resolve_remote(yt_dlp, watch_url)
                .await
                .map_err(|e| format!("{first}; re-resolve 失敗: {e}"))?;
            if fresh.is_live {
                return Err(format!("{first}; re-resolve 發現來源已轉為直播"));
            }
            let pb = fresh
                .audio_url
                .or(fresh.playback_url)
                .ok_or_else(|| format!("{first}; re-resolve 無可用音訊來源"))?;
            decode_remote_audio(ffmpeg, &pb, &fresh.http_headers, dest).await
        }
    }
}

/// `yt-dlp -f ba` 下載完整音訊到 dest 同目錄暫存檔（副檔名由 yt-dlp 決定），再 ffmpeg 解成 16k mono wav；清暫存。
async fn download_audio_then_decode(
    ffmpeg: &Path,
    yt_dlp: &Path,
    watch_url: &str,
    dest: &Path,
) -> Result<PathBuf, String> {
    let dir = dest.parent().ok_or("dest 無上層目錄")?;
    let stem = dest.file_stem().and_then(|s| s.to_str()).ok_or("dest 無檔名")?;
    let out_tmpl = dir.join(format!("{stem}.dl.%(ext)s"));
    let mut cmd = Command::new(yt_dlp);
    cmd.args(["-f", "ba", "--no-playlist", "-o"])
        .arg(&out_tmpl)
        .arg(watch_url)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    no_window(&mut cmd);
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "yt-dlp -f ba exit: {}",
            String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")
        ));
    }
    let downloaded =
        find_one_with_prefix(dir, &format!("{stem}.dl.")).ok_or("找不到下載的音訊檔")?;
    let decoded = decode_local_to_wav(ffmpeg, &downloaded, dest).await;
    let _ = std::fs::remove_file(&downloaded); // 清暫存（不論成敗）
    decoded
}

/// ffmpeg 把本地音訊檔解成 16k mono s16 wav。
async fn decode_local_to_wav(ffmpeg: &Path, src: &Path, dest: &Path) -> Result<PathBuf, String> {
    let mut cmd = Command::new(ffmpeg);
    cmd.arg("-i")
        .arg(src)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav", "-y"])
        .arg(dest)
        .kill_on_drop(true);
    no_window(&mut cmd);
    let st = cmd.status().await.map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(format!("ffmpeg 解碼本地音訊 exit {st}"));
    }
    Ok(dest.to_path_buf())
}

/// 找 dir 下檔名以 prefix 開頭、且非 .part 的第一個檔（yt-dlp 下載完成後的成品）。
fn find_one_with_prefix(dir: &Path, prefix: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(prefix) && !n.ends_with(".part"))
                .unwrap_or(false)
        })
}

fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

async fn ytdlp_get_url(yt_dlp: &Path, url: &str, fmt: &str) -> Option<String> {
    let mut cmd = Command::new(yt_dlp);
    cmd.args(["-f", fmt, "-g", "--no-playlist", url]).stdout(Stdio::piped()).stderr(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().await.ok()?;
    if !out.status.success() { return None; }
    String::from_utf8_lossy(&out.stdout).lines().next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

const RESOLVE_MAX_ATTEMPTS: usize = 3;

/// range-GET 探測 playback_url 是否可達（帶 pick_resolved 的 headers，含 UA）。
/// 200/206（success）→ true；403/其他狀態/網路錯 → false。保留 reqwest 預設 follow-redirect。
async fn probe_playback_url(http: &reqwest::Client, url: &str, headers: &BTreeMap<String, String>) -> bool {
    let mut req = http.get(url).header("Range", "bytes=0-1");
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    req.send().await.is_ok_and(|resp| resp.status().is_success())
}

/// Resolve a remote URL，並探測 playback_url；403/不可達則 re-resolve（yt-dlp 重出新簽名 URL）重試。
/// ≤RESOLVE_MAX_ATTEMPTS 次；首個探測通過即回；全失敗 → best-effort 回最後一個（≥ 現狀、不回歸）。
/// 直播早回（不探測）。client 建立失敗（現實不會）→ 無探測模式（視同通過）。
pub async fn resolve_remote(yt_dlp: &Path, url: &str) -> Result<Picked, String> {
    let http = reqwest::Client::builder().timeout(Duration::from_secs(5)).build().ok();
    let mut last: Option<Picked> = None;
    for _ in 0..RESOLVE_MAX_ATTEMPTS {
        let p = resolve_once(yt_dlp, url).await?;
        if p.is_live {
            return Ok(p);
        }
        let pb = p
            .playback_url
            .clone()
            .expect("resolve_once 回 Ok 且非 live 時 playback_url 必為 Some");
        let ok = match &http {
            Some(c) => probe_playback_url(c, &pb, &p.http_headers).await,
            None => true,
        };
        if ok {
            return Ok(p);
        }
        last = Some(p);
    }
    Ok(last.expect("RESOLVE_MAX_ATTEMPTS>=1，迴圈跑滿時 last 必為 Some"))
}

/// 單次解析：spawn `yt-dlp -J` → pick_resolved → fill missing single-url via `-g`。
async fn resolve_once(yt_dlp: &Path, url: &str) -> Result<Picked, String> {
    let mut cmd = Command::new(yt_dlp);
    cmd.args(["-J", "--no-playlist", url]).stdout(Stdio::piped()).stderr(Stdio::piped());
    no_window(&mut cmd);
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("yt-dlp 解析失敗: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    let j: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut p = pick_resolved(&j);
    if p.is_live {
        return Ok(p); // caller blocks live; keep is_live=true + the rest
    }
    // NOTE(Phase 2 follow-up): URLs filled here via `-g` do NOT refresh `http_headers`
    // (those stay whatever pick_resolved derived from the JSON formats). Benign for YouTube
    // (formats share the client User-Agent), but a `-g`-only URL could 403 if headers mismatch.
    // When Phase 2 wires per-window decode, capture the chosen format's headers here.
    if p.playback_url.is_none() {
        p.playback_url = ytdlp_get_url(yt_dlp, url, "best").await;
    }
    if p.audio_url.is_none() {
        p.audio_url = ytdlp_get_url(yt_dlp, url, "bestaudio").await;
    }
    if p.playback_url.is_none() {
        return Err("無可用影音直連 URL".into());
    }
    Ok(p)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatEntry {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlatPlaylist {
    pub title: Option<String>,
    pub entries: Vec<FlatEntry>,
}

/// 合法 YouTube videoId = 11 字 [A-Za-z0-9_-]。
fn is_video_id(s: &str) -> bool {
    s.len() == 11 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// 解析 `yt-dlp --flat-playlist -J` 輸出：只留 `_type=="url"` 且 id 為合法 videoId 的項；
/// title 缺則 fallback 成 id。非清單 JSON（無 entries）→ 空列。
pub fn parse_flat_playlist(j: &serde_json::Value) -> FlatPlaylist {
    let title = j.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
    let entries = j
        .get("entries")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter(|e| e.get("_type").and_then(|t| t.as_str()) == Some("url"))
                .filter_map(|e| {
                    let id = e.get("id").and_then(|v| v.as_str())?;
                    if !is_video_id(id) {
                        return None;
                    }
                    let title = e
                        .get("title")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(id)
                        .to_string();
                    Some(FlatEntry { id: id.to_string(), title })
                })
                .collect()
        })
        .unwrap_or_default();
    FlatPlaylist { title, entries }
}

/// 枚舉清單（僅 meta，不解析串流）。失敗回 Err。
pub async fn enumerate_playlist(yt_dlp: &Path, url: &str) -> Result<FlatPlaylist, String> {
    let mut cmd = Command::new(yt_dlp);
    cmd.args(["--flat-playlist", "-J", url]).stdout(Stdio::piped()).stderr(Stdio::piped()); // 不可加 --no-playlist
    no_window(&mut cmd);
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("yt-dlp 枚舉清單失敗: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    let j: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    Ok(parse_flat_playlist(&j))
}

/// 從 yt-dlp -J JSON 取 title（非空）。純函式。
pub fn title_from_json(j: &serde_json::Value) -> Option<String> {
    j.get("title").and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

/// 取標題：yt-dlp -J --no-playlist → serde 解析 title。
/// 走 -J/JSON（與 resolve_remote 同路徑、編碼正確）而非 `--print` 原始文字——後者在 Windows
/// 被 pipe 時 yt-dlp(Python) 以系統 codepage 輸出、CJK 在 yt-dlp 端就變字面 "?"，連 PYTHONIOENCODING 也壓不住。
/// 失敗/非零退出/無 title → None（best-effort，呼叫端回退 URL）。
pub async fn fetch_remote_title(yt_dlp: &Path, url: &str) -> Option<String> {
    let mut cmd = Command::new(yt_dlp);
    cmd.args(["-J", "--no-playlist", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    no_window(&mut cmd);
    let out = cmd.output().await.ok()?;
    if !out.status.success() {
        return None;
    }
    let j: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    title_from_json(&j)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_flat_playlist_filters_non_videos_and_keeps_order() {
        let j = json!({
            "title": "My List",
            "entries": [
                {"_type":"url","id":"abcdefghijk","title":"First"},
                {"_type":"url","id":"chan","title":"A Channel"},          // 非 11 字 id → 丟
                {"_type":"url","id":"LMNOPQRSTUV","title":null},          // title 缺 → fallback id
                {"_type":"playlist","id":"xxxxxxxxxxx","title":"Sub"}     // 非 url → 丟
            ]
        });
        let fp = parse_flat_playlist(&j);
        assert_eq!(fp.title.as_deref(), Some("My List"));
        assert_eq!(fp.entries.len(), 2);
        assert_eq!(fp.entries[0].id, "abcdefghijk");
        assert_eq!(fp.entries[0].title, "First");
        assert_eq!(fp.entries[1].id, "LMNOPQRSTUV");
        assert_eq!(fp.entries[1].title, "LMNOPQRSTUV"); // fallback
    }

    #[test]
    fn parse_flat_playlist_empty() {
        let fp = parse_flat_playlist(&json!({"entries": []}));
        assert!(fp.entries.is_empty());
    }

    #[test]
    fn picks_single_url_audio_and_combined() {
        let j = json!({
            "title": "醉", "duration": 600.0, "is_live": false,
            "formats": [
                {"vcodec":"none","acodec":"opus","protocol":"https","url":"A1","tbr":50.0,
                 "http_headers":{"User-Agent":"UA","Accept-Language":"en"}},
                {"vcodec":"none","acodec":"opus","protocol":"https","url":"A2","tbr":130.0,
                 "http_headers":{"User-Agent":"UA"}},
                {"vcodec":"avc1","acodec":"mp4a","protocol":"https","url":"V1","tbr":700.0,
                 "http_headers":{"User-Agent":"UA"}}
            ]
        });
        let p = pick_resolved(&j);
        assert_eq!(p.audio_url.as_deref(), Some("A2"));
        assert_eq!(p.playback_url.as_deref(), Some("V1"));
        assert_eq!(p.duration_sec, 600.0);
        assert!(!p.is_live);
        assert_eq!(p.http_headers.get("User-Agent").map(|s| s.as_str()), Some("UA"));
        assert_eq!(p.title.as_deref(), Some("醉"));
    }

    #[test]
    fn dash_only_yields_none_urls() {
        let j = json!({
            "duration": 100.0,
            "formats": [
                {"vcodec":"none","acodec":"opus","protocol":"http_dash_segments","fragments":[{"path":"x"}]},
                {"vcodec":"avc1","acodec":"none","protocol":"http_dash_segments","fragments":[{"path":"y"}]}
            ]
        });
        let p = pick_resolved(&j);
        assert_eq!(p.audio_url, None);
        assert_eq!(p.playback_url, None);
        assert!(p.title.is_none()); // json 無 title → None
    }

    #[test]
    fn detects_live_and_missing_duration() {
        let j = json!({ "is_live": true, "formats": [] });
        let p = pick_resolved(&j);
        assert!(p.is_live);
        assert_eq!(p.duration_sec, 0.0);
        assert!(p.audio_url.is_none());
    }

    #[test]
    fn collects_vtt_cc_only() {
        let j = json!({
            "formats": [],
            "subtitles": { "en": [ {"ext":"json3","url":"J"}, {"ext":"vtt","url":"EN_VTT"} ] },
            "automatic_captions": { "ja": [ {"ext":"vtt","url":"JA_VTT"} ], "ko": [ {"ext":"srv1","url":"S"} ] }
        });
        let p = pick_resolved(&j);
        assert_eq!(p.cc_tracks.len(), 2);
        assert!(p.cc_tracks.iter().any(|c| c.vtt_url == "EN_VTT" && !c.auto));
        assert!(p.cc_tracks.iter().any(|c| c.vtt_url == "JA_VTT" && c.auto));
    }

    fn bmap(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn fetch_args_extract_ua_and_headers() {
        let h = bmap(&[("User-Agent", "UA/1.0"), ("Accept-Language", "en")]);
        let a = build_remote_decode_args("PB", &h, "out.wav");
        let i = a.iter().position(|s| s == "-user_agent").expect("has -user_agent");
        assert_eq!(a[i + 1], "UA/1.0");
        let j = a.iter().position(|s| s == "-headers").expect("has -headers");
        assert_eq!(a[j + 1], "Accept-Language: en\r\n");
        let pb = a.iter().position(|s| s == "-i").unwrap();
        assert_eq!(a[pb + 1], "PB");
        assert!(a.iter().any(|s| s == "-vn"));
        assert!(a.iter().any(|s| s == "-y"));
        assert_eq!(a.last().map(|s| s.as_str()), Some("out.wav"));
    }

    #[test]
    fn fetch_args_ua_key_case_insensitive() {
        let h = bmap(&[("user-agent", "lc")]);
        let a = build_remote_decode_args("PB", &h, "o.wav");
        let i = a.iter().position(|s| s == "-user_agent").expect("has -user_agent");
        assert_eq!(a[i + 1], "lc");
        assert!(!a.iter().any(|s| s == "-headers"));
    }

    #[test]
    fn fetch_args_multi_header_crlf_joined() {
        let h = bmap(&[("X-A", "1"), ("X-B", "2")]); // 無 UA
        let a = build_remote_decode_args("PB", &h, "o.wav");
        assert!(!a.iter().any(|s| s == "-user_agent"));
        let j = a.iter().position(|s| s == "-headers").unwrap();
        // BTreeMap 排序 → X-A 在 X-B 前
        assert_eq!(a[j + 1], "X-A: 1\r\nX-B: 2\r\n");
    }

    #[test]
    fn fetch_args_empty_headers_omits_flags() {
        let h: BTreeMap<String, String> = BTreeMap::new();
        let a = build_remote_decode_args("PB", &h, "o.wav");
        assert!(!a.iter().any(|s| s == "-headers"));
        assert!(!a.iter().any(|s| s == "-user_agent"));
        assert!(a.iter().any(|s| s == "-y"));
        assert!(a.iter().any(|s| s == "-vn"));
        assert_eq!(a.iter().position(|s| s == "-i").map(|p| a[p + 1].as_str()), Some("PB"));
    }

    #[test]
    fn title_from_json_present_incl_cjk() {
        assert_eq!(title_from_json(&json!({"title": "漂亮 video"})), Some("漂亮 video".to_string()));
    }

    #[test]
    fn title_from_json_empty_is_none() {
        assert_eq!(title_from_json(&json!({"title": ""})), None);
    }

    #[test]
    fn title_from_json_missing_is_none() {
        assert_eq!(title_from_json(&json!({"id": "x"})), None);
    }

    #[test]
    fn videos_prefers_av1_per_height() {
        let j = serde_json::json!({
            "duration": 100.0, "is_live": false, "title": "t",
            "formats": [
                {"format_id":"248","protocol":"https","url":"https://v/1080vp9","vcodec":"vp9","acodec":"none","height":1080,"fps":30,"tbr":1543.0},
                {"format_id":"399","protocol":"https","url":"https://v/1080av1","vcodec":"av01.0.08M.08","acodec":"none","height":1080,"fps":30,"tbr":1162.0},
                {"format_id":"313","protocol":"https","url":"https://v/2160vp9","vcodec":"vp9","acodec":"none","height":2160,"fps":30,"tbr":10101.0},
                {"format_id":"251","protocol":"https","url":"https://a/opus","vcodec":"none","acodec":"opus","tbr":108.0},
                {"format_id":"18","protocol":"https","url":"https://m/muxed","vcodec":"avc1","acodec":"mp4a","height":360,"fps":30,"tbr":410.0}
            ]
        });
        let p = pick_resolved(&j);
        let heights: Vec<u32> = p.videos.iter().map(|v| v.height).collect();
        assert_eq!(heights, vec![2160, 1080]);
        let v1080 = p.videos.iter().find(|v| v.height == 1080).unwrap();
        assert_eq!(v1080.itag, "399");
        assert_eq!(v1080.url, "https://v/1080av1");
        assert!(p.audio_url.is_some());
    }

    #[test]
    fn videos_same_codec_breaks_tie_by_max_tbr() {
        // 同 height 同編碼器（皆 vp9）→ tbr 高者勝（覆蓋 and_modify 的第二子句）。
        let j = serde_json::json!({
            "duration": 100.0, "is_live": false,
            "formats": [
                {"format_id":"247","protocol":"https","url":"https://v/720lo","vcodec":"vp9","acodec":"none","height":720,"fps":30,"tbr":800.0},
                {"format_id":"302","protocol":"https","url":"https://v/720hi","vcodec":"vp9","acodec":"none","height":720,"fps":30,"tbr":1200.0}
            ]
        });
        let p = pick_resolved(&j);
        let v720 = p.videos.iter().find(|v| v.height == 720).unwrap();
        assert_eq!(v720.itag, "302");
        assert_eq!(v720.url, "https://v/720hi");
    }

    #[test]
    fn videos_empty_when_only_muxed() {
        let j = serde_json::json!({
            "duration": 10.0, "is_live": false,
            "formats": [{"format_id":"18","protocol":"https","url":"https://m/x","vcodec":"avc1","acodec":"mp4a","height":360,"tbr":410.0}]
        });
        let p = pick_resolved(&j);
        assert!(p.videos.is_empty());
        assert_eq!(p.playback_url.as_deref(), Some("https://m/x"));
    }
}
