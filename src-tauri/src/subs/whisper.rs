use super::merge::RawSeg;
use super::session::VadParams;
use super::stream::{StreamSeg, WordTs};
use std::ffi::OsString;
use std::net::TcpListener;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};

pub fn free_port() -> Result<u16, String> {
    let l = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    Ok(l.local_addr().map_err(|e| e.to_string())?.port())
}

/// 純函式：組 whisper-server 啟動旗標（可單測；start() 呼叫它）。
/// 回 Vec<OsString>（model/vad_model 為 &Path、可能含非 UTF-8）。
/// 注意：與 remote.rs::build_remote_decode_args（回 Vec<String>）僅「純函式/可測」精神對齊，回傳型別不同。
pub fn build_server_args(model: &Path, vad_model: &Path, vad: &VadParams, port: u16) -> Vec<OsString> {
    let mut args = vec![OsString::from("-m"), model.into()];
    if vad.vad_enabled {
        let vt = format!("{:.2}", vad.threshold);
        let vsd = vad.min_silence_ms.to_string();
        args.extend([
            OsString::from("--vad"),
            OsString::from("-vm"), vad_model.into(),
            OsString::from("--vad-threshold"), OsString::from(vt),
            OsString::from("--vad-min-silence-duration-ms"), OsString::from(vsd),
        ]);
    }
    args.extend([
        OsString::from("--suppress-nst"),
        OsString::from("--host"), OsString::from("127.0.0.1"),
        OsString::from("--port"), OsString::from(port.to_string()),
    ]);
    args
}

/// 長壽常駐 whisper-server（由 Manager 持有、跨 session 重用）。
pub struct WhisperServer {
    child: Child,
    pub port: u16,
}

impl WhisperServer {
    pub async fn start(exe: &Path, model: &Path, vad_model: &Path, vad_params: &VadParams) -> Result<Self, String> {
        let port = free_port()?;
        let mut cmd = Command::new(exe);
        cmd.args(build_server_args(model, vad_model, vad_params, port))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：whisper-server 不彈 console 視窗
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let http = reqwest::Client::new();
        for _ in 0..120 {
            // 等模型載入 ~60s
            if let Ok(Some(st)) = child.try_wait() {
                return Err(format!("whisper-server exited early: {st}"));
            }
            if http.get(format!("http://127.0.0.1:{port}/")).send().await.is_ok() {
                return Ok(Self { child, port });
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let _ = child.kill().await;
        Err("whisper-server health timeout".into())
    }

    /// 明確終止並回收（停用/關 app 時呼叫，不依賴 kill_on_drop）。
    pub async fn kill(mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

/// 送視窗 wav 到 port，回 (段落[視窗相對秒], 偵測語言 ISO)。schema 由 Task 9 probe 確認。
pub async fn transcribe(
    http: &reqwest::Client,
    port: u16,
    wav: &Path,
    lang: Option<&str>,
    prompt: Option<&str>,
) -> Result<(Vec<RawSeg>, Option<String>), String> {
    let bytes = tokio::fs::read(wav).await.map_err(|e| e.to_string())?;
    let part = reqwest::multipart::Part::bytes(bytes).file_name("a.wav");
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("response_format", "verbose_json");
    if let Some(l) = lang {
        form = form.text("language", l.to_string());
    }
    if let Some(p) = prompt {
        form = form.text("prompt", p.to_string());
    }
    let resp = http
        .post(format!("http://127.0.0.1:{port}/inference"))
        .multipart(form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // Task 9 probe 確認 verbose_json：頂層 "language" 是全名（"english"）→ 不可回傳當參數；
    // 取 "language_probabilities" 的 argmax 拿 ISO 碼（"en"）。
    let lang_out = v
        .get("language_probabilities")
        .and_then(|x| x.as_object())
        .and_then(|m| {
            m.iter()
                .filter_map(|(k, p)| p.as_f64().map(|p| (k.clone(), p)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(k, _)| k)
        });
    // segments[].{start,end}（秒,f64）、text — probe 確認。
    let segs = v
        .get("segments")
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| {
                    Some(RawSeg {
                        start_sec: s.get("start")?.as_f64()?,
                        end_sec: s.get("end")?.as_f64()?,
                        text: s.get("text")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok((segs, lang_out))
}

/// 16k mono i16 PCM → 最小 WAV bytes（不落地）。
#[allow(dead_code)]
pub fn pcm16_wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());       // PCM
    b.extend_from_slice(&1u16.to_le_bytes());       // mono
    b.extend_from_slice(&16000u32.to_le_bytes());   // sample rate
    b.extend_from_slice(&(16000u32 * 2).to_le_bytes()); // byte rate
    b.extend_from_slice(&2u16.to_le_bytes());       // block align
    b.extend_from_slice(&16u16.to_le_bytes());      // bits/sample
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples { b.extend_from_slice(&s.to_le_bytes()); }
    b
}

/// f32 [-1,1] → i16。
#[allow(dead_code)]
pub fn f32_to_i16(frames: &[f32]) -> Vec<i16> {
    frames.iter().map(|x| (x.clamp(-1.0, 1.0) * 32767.0) as i16).collect()
}

/// 解析 verbose_json segments → StreamSeg（含 no_speech_prob + words）。
#[allow(dead_code)]
pub fn parse_stream_segs(v: &serde_json::Value) -> Vec<StreamSeg> {
    v.get("segments").and_then(|x| x.as_array()).map(|a| {
        a.iter().filter_map(|s| {
            let words = s.get("words").and_then(|w| w.as_array()).map(|arr| {
                arr.iter().filter_map(|w| Some(WordTs {
                    word: w.get("word")?.as_str()?.to_string(),
                    start_sec: w.get("start")?.as_f64()?,
                    end_sec: w.get("end")?.as_f64()?,
                })).collect()
            }).unwrap_or_default();
            Some(StreamSeg {
                start_sec: s.get("start")?.as_f64()?,
                end_sec: s.get("end")?.as_f64()?,
                text: s.get("text")?.as_str()?.to_string(),
                no_speech_prob: s.get("no_speech_prob").and_then(|x| x.as_f64()).unwrap_or(0.0),
                words,
            })
        }).collect()
    }).unwrap_or_default()
}

/// 送 in-memory 16k mono f32 視窗 → 回 StreamSeg。lang 必為 Some（§4.7）。
#[allow(dead_code)]
pub async fn transcribe_bytes(
    http: &reqwest::Client, port: u16, frames: &[f32], lang: &str, prompt: Option<&str>,
) -> Result<Vec<StreamSeg>, String> {
    let wav = pcm16_wav_bytes(&f32_to_i16(frames));
    let part = reqwest::multipart::Part::bytes(wav).file_name("w.wav");
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("response_format", "verbose_json")
        .text("language", lang.to_string());
    if let Some(p) = prompt { form = form.text("prompt", p.to_string()); }
    let resp = http.post(format!("http://127.0.0.1:{port}/inference"))
        .multipart(form).send().await.map_err(|e| e.to_string())?;
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parse_stream_segs(&v))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::path::Path;

    fn sample_args() -> Vec<OsString> {
        let vad = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: true };
        build_server_args(Path::new("model.bin"), Path::new("vad.bin"), &vad, 12345)
    }

    // args 內含某單一 token（如布林裸旗）
    fn has(args: &[OsString], needle: &str) -> bool {
        args.contains(&OsString::from(needle))
    }

    // args 內含相鄰子序列 [a, b]（如 旗標+值），不綁死整體順序
    fn has_pair(args: &[OsString], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| {
            w[0].as_os_str() == OsStr::new(a) && w[1].as_os_str() == OsStr::new(b)
        })
    }

    #[test]
    fn build_server_args_includes_suppress_nst() {
        // 布林裸旗：單一 token，不接值
        assert!(has(&sample_args(), "--suppress-nst"));
    }

    #[test]
    fn build_server_args_keeps_existing_flags() {
        let args = sample_args();
        assert!(has_pair(&args, "-m", "model.bin"), "缺 -m <model>");
        assert!(has(&args, "--vad"), "缺 --vad 布林旗");
        assert!(has_pair(&args, "-vm", "vad.bin"), "缺 -vm <vad_model>");
        assert!(has_pair(&args, "--vad-threshold", "0.50"), "缺/錯 --vad-threshold");
        assert!(has_pair(&args, "--vad-min-silence-duration-ms", "100"), "缺/錯 vad-min-silence");
        assert!(has_pair(&args, "--port", "12345"), "缺/錯 --port");
    }

    #[test]
    fn vad_off_omits_vad_flags() {
        let vad = VadParams { threshold: 0.5, min_silence_ms: 100, vad_enabled: false };
        let a = build_server_args(Path::new("m.bin"), Path::new("v.bin"), &vad, 9);
        assert!(!has(&a, "--vad"));
        assert!(!has_pair(&a, "-vm", "v.bin"));
        assert!(has(&a, "--suppress-nst"));
    }

    #[test]
    fn wav_header_pcm16_16k_mono() {
        let pcm = pcm16_wav_bytes(&[0i16, 1, -1]);
        assert_eq!(&pcm[0..4], b"RIFF");
        assert_eq!(&pcm[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes([pcm[24], pcm[25], pcm[26], pcm[27]]), 16000);
    }

    #[test]
    fn parse_stream_segs_reads_no_speech_and_words() {
        let v: serde_json::Value = serde_json::from_str(r#"{
          "segments":[{"start":0.0,"end":2.0,"text":"hi","no_speech_prob":0.12,
            "words":[{"word":" hi","start":0.0,"end":0.5}]}]}"#).unwrap();
        let segs = parse_stream_segs(&v);
        assert_eq!(segs.len(), 1);
        assert!((segs[0].no_speech_prob - 0.12).abs() < 1e-9);
        assert_eq!(segs[0].words.len(), 1);
        assert_eq!(segs[0].words[0].word, " hi");
    }
}
