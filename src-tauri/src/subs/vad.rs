use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

/// 解析 whisper-vad-speech-segments.exe 的 stdout。
/// 行格式（實證）：`Speech segment K: start = X, end = Y`，X/Y 單位＝**centiseconds（1/100 秒）**。
/// 回 (start_sec, end_sec)（已 ÷100 轉秒），過濾 end<=start 與壞行。
pub fn parse_vad_segments(stdout: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let l = line.trim();
        let Some(rest) = l.strip_prefix("Speech segment ") else { continue };
        let Some(si) = rest.find("start = ") else { continue };
        let after = &rest[si + "start = ".len()..];
        let Some(comma) = after.find(',') else { continue };
        let Some(ei) = after.find("end = ") else { continue };
        let start_s = after[..comma].trim();
        let end_s = after[ei + "end = ".len()..].trim();
        if let (Ok(s), Ok(e)) = (start_s.parse::<f64>(), end_s.parse::<f64>()) {
            let (s, e) = (s / 100.0, e / 100.0);
            if e > s {
                out.push((s, e));
            }
        }
    }
    out
}

/// 視窗為半開區間 [start, end)：段須與其有 > EPS 的實際重疊才算「窗內有語音」。
/// 起點落在窗尾(或其後)、終點落在窗起點(或其前)的段屬相鄰窗，不算——
/// 否則對齊後 end==段首 的純靜音窗會被誤判有語音、送進 --vad whisper-server 觸發 0-speech 崩潰。
pub fn window_has_speech(speech: &[(f64, f64)], start: f64, end: f64) -> bool {
    const EPS: f64 = 0.05;
    speech.iter().any(|(a, b)| *a + EPS < end && start + EPS < *b)
}

/// 組 whisper-vad-speech-segments.exe 參數（純函式可測）。**CPU：不加 -ug**
/// （實證：-ug 對整軌崩 0xC0000409；CPU 整首 4:35 僅 0.67s）。
pub fn build_vad_args(vad_model: &Path, track_wav: &Path, threshold: f64, min_silence_ms: i64) -> Vec<OsString> {
    let vt = format!("{:.2}", threshold);
    let vsd = min_silence_ms.to_string();
    vec![
        OsString::from("-f"), track_wav.into(),
        OsString::from("-vm"), vad_model.into(),
        OsString::from("-vt"), OsString::from(vt),
        OsString::from("-vsd"), OsString::from(vsd),
    ]
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW（tokio Command 固有方法，比照 remote.rs）
}
#[cfg(not(windows))]
fn no_window(_cmd: &mut Command) {}

/// 對整軌 wav 跑 whisper-vad-speech-segments.exe（CPU）→ 語音段（秒）。
/// 「Detected/Speech segment」行走 stdout（實證）；診斷行走 stderr（丟棄）。
pub async fn detect_speech_segments(
    vad_exe: &Path,
    vad_model: &Path,
    track_wav: &Path,
    threshold: f64,
    min_silence_ms: i64,
) -> Result<Vec<(f64, f64)>, String> {
    let mut cmd = Command::new(vad_exe);
    cmd.args(build_vad_args(vad_model, track_wav, threshold, min_silence_ms))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    no_window(&mut cmd);
    let out = cmd.output().await.map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("whisper-vad exit {}", out.status));
    }
    Ok(parse_vad_segments(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::Path;

    #[test]
    fn parse_multi_segments_centiseconds_to_seconds() {
        let s = "Detected 2 speech segments:\n\
                 Speech segment 0: start = 5626.00, end = 5683.00\n\
                 Speech segment 1: start = 25264.00, end = 25686.00\n";
        let v = parse_vad_segments(s);
        assert_eq!(v.len(), 2);
        assert!((v[0].0 - 56.26).abs() < 1e-6, "start ÷100");
        assert!((v[0].1 - 56.83).abs() < 1e-6);
        assert!((v[1].0 - 252.64).abs() < 1e-6);
        assert!((v[1].1 - 256.86).abs() < 1e-6);
    }

    #[test]
    fn parse_zero_segments_and_empty() {
        assert!(parse_vad_segments("Detected 0 speech segments:\n").is_empty());
        assert!(parse_vad_segments("").is_empty());
    }

    #[test]
    fn parse_filters_bad_and_noise() {
        let s = "whisper_vad_init: noise line\n\
                 Speech segment 0: start = 100.00, end = 50.00\n\
                 Speech segment 1: start = 100.00, end = 200.00\n";
        // 第一段 end<start → 濾掉；第二段 (1.0,2.0)
        assert_eq!(parse_vad_segments(s), vec![(1.0, 2.0)]);
    }

    #[test]
    fn window_has_speech_cases() {
        let sp = vec![(56.26, 56.83), (60.0, 90.0)];
        assert!(!window_has_speech(&sp, 0.0, 28.0), "器樂前奏無語音");
        assert!(!window_has_speech(&sp, 26.0, 56.0), "緊鄰但未到語音");
        assert!(window_has_speech(&sp, 54.0, 84.0), "重疊 56.26 與 60-90");
        assert!(window_has_speech(&sp, 88.0, 116.0), "重疊 60-90 尾");
        assert!(!window_has_speech(&sp, 90.1, 118.0), "全部語音之後");
    }

    #[test]
    fn window_has_speech_half_open_boundaries() {
        // 段起點 == 窗尾 → 不算（原 BLOCKER：對齊後 [start,a] 純靜音窗須可跳過）
        assert!(!window_has_speech(&[(50.0, 90.0)], 26.0, 50.0));
        // 段終點 == 窗起點 → 不算（對稱半開）
        assert!(!window_has_speech(&[(0.0, 26.0)], 26.0, 54.0));
        // 窗內有實際語音 → 算
        assert!(window_has_speech(&[(1.0, 4.0), (26.0, 33.0)], 0.0, 26.0));
    }

    #[test]
    fn build_vad_args_cpu_no_ug() {
        let a = build_vad_args(Path::new("vad.bin"), Path::new("t.wav"), 0.5, 100);
        let pair = |x: &str, y: &str| {
            a.windows(2).any(|w| w[0].as_os_str() == OsStr::new(x) && w[1].as_os_str() == OsStr::new(y))
        };
        assert!(pair("-f", "t.wav"));
        assert!(pair("-vm", "vad.bin"));
        assert!(pair("-vt", "0.50"));
        assert!(pair("-vsd", "100"));
        // 單一 token 檢查用 contains（避開 clippy::manual_contains）
        assert!(!a.contains(&OsString::from("-ug")), "必須 CPU、不得有 -ug");
    }
}
