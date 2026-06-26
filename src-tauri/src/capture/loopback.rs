#![allow(dead_code)]

use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use crate::capture::source::AudioSource;

pub const TARGET_RATE: u32 = 16000;
/// 能量門檻（型別 f32 配 tail_has_speech；實機 probe 可調）。
pub const ENERGY_RMS_THRESH: f32 = 0.01;

/// 交錯多聲道 f32 → 單聲道（各 frame 取平均）。channels>=1。
pub fn downmix(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 { return interleaved.to_vec(); }
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// 單聲道 f32 從 src_rate 重取樣到 16k。src_rate==16000 直接回傳。
pub fn resample_to_16k(mono: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == TARGET_RATE || mono.is_empty() { return mono.to_vec(); }
    let ratio = TARGET_RATE as f64 / src_rate as f64;
    let params = SincInterpolationParameters {
        sinc_len: 128, f_cutoff: 0.95, oversampling_factor: 128,
        interpolation: SincInterpolationType::Linear, window: WindowFunction::BlackmanHarris2,
    };
    let mut rs = match SincFixedIn::<f32>::new(ratio, 2.0, params, mono.len(), 1) {
        Ok(r) => r, Err(_) => return linear_decimate(mono, src_rate),
    };
    match rs.process(&[mono.to_vec()], None) {
        Ok(mut out) => out.pop().unwrap_or_default(),
        Err(_) => linear_decimate(mono, src_rate),
    }
}

/// 退路：線性插值重取樣（rubato 失敗時）。
fn linear_decimate(mono: &[f32], src_rate: u32) -> Vec<f32> {
    let ratio = src_rate as f64 / TARGET_RATE as f64;
    let n = (mono.len() as f64 / ratio).floor() as usize;
    (0..n).map(|i| {
        let pos = i as f64 * ratio;
        let j = pos.floor() as usize;
        let frac = pos - j as f64;
        let a = mono[j.min(mono.len() - 1)];
        let b = mono[(j + 1).min(mono.len() - 1)];
        a + (b - a) * frac as f32
    }).collect()
}

/// 一段 16k mono 的 RMS。
pub fn rms(frames: &[f32]) -> f32 {
    if frames.is_empty() { return 0.0; }
    (frames.iter().map(|x| x * x).sum::<f32>() / frames.len() as f32).sqrt()
}

/// 窗尾 tail_sec 是否「有語音」（RMS 超門檻）。
pub fn tail_has_speech(frames: &[f32], tail_sec: f64, thresh: f32) -> bool {
    let n = ((tail_sec * TARGET_RATE as f64) as usize).min(frames.len());
    if n == 0 { return false; }
    rms(&frames[frames.len() - n..]) >= thresh
}

// ─────────────────────────── Task 8: 裝置列舉 ───────────────────────────

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSources {
    pub processes: Vec<crate::capture::sessions::ProcessSource>,
    pub input_devices: Vec<AudioDevice>,
}

/// 列出 capture（麥克風/輸入）裝置（wasapi 0.19）。必須在 spawn_blocking 內呼叫（COM）。
pub fn list_input_devices() -> Result<Vec<AudioDevice>, String> {
    let _ = wasapi::initialize_mta();
    let default_id = wasapi::get_default_device(&wasapi::Direction::Capture)
        .ok()
        .and_then(|d| d.get_id().ok());
    let collection = wasapi::DeviceCollection::new(&wasapi::Direction::Capture)
        .map_err(|e| format!("列舉 capture 裝置失敗: {e}"))?;
    let count = collection.get_nbr_devices().map_err(|e| e.to_string())?;
    let mut devices = Vec::new();
    for i in 0..count {
        let dev = match collection.get_device_at_index(i) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let id = match dev.get_id() {
            Ok(s) => s,
            Err(_) => continue,
        };
        let name = dev.get_friendlyname().unwrap_or_else(|_| id.clone());
        let is_default = default_id.as_deref() == Some(id.as_str());
        devices.push(AudioDevice { id, name, is_default });
    }
    Ok(devices)
}

// ─────────────────────── Task 9: loopback 擷取執行緒 ───────────────────────

/// loopback 擷取啟動回傳：(停止旗標, 執行緒 handle, 16k mono f32 chunk receiver)。
type CaptureSession = (Arc<AtomicBool>, std::thread::JoinHandle<()>, Receiver<Vec<f32>>);

/// 啟動音源擷取執行緒。
/// 回 (stop_flag, JoinHandle, Receiver<16k mono f32 chunks>)。
pub fn start_capture(source: AudioSource) -> Result<CaptureSession, String> {
    let (tx, rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    let handle = std::thread::spawn(move || {
        if let Err(e) = capture_loop(source, tx, &stop2) {
            eprintln!("[loopback] capture loop ended: {e}");
        }
    });
    Ok((stop, handle, rx))
}

/// 依 id 找 render 裝置（在已 COM-init 的執行緒內呼叫）。
fn find_render_device_by_id(id: &str) -> Result<wasapi::Device, String> {
    let collection = wasapi::DeviceCollection::new(&wasapi::Direction::Render)
        .map_err(|e| e.to_string())?;
    let count = collection.get_nbr_devices().map_err(|e| e.to_string())?;
    for i in 0..count {
        if let Ok(dev) = collection.get_device_at_index(i) {
            if dev.get_id().ok().as_deref() == Some(id) {
                return Ok(dev);
            }
        }
    }
    Err(format!("找不到 render 裝置 id={id}"))
}

/// 依 id 找 capture（輸入）裝置（在已 COM-init 的執行緒內呼叫）。
fn find_capture_device_by_id(id: &str) -> Result<wasapi::Device, String> {
    let collection = wasapi::DeviceCollection::new(&wasapi::Direction::Capture)
        .map_err(|e| e.to_string())?;
    let count = collection.get_nbr_devices().map_err(|e| e.to_string())?;
    for i in 0..count {
        if let Ok(dev) = collection.get_device_at_index(i) {
            if dev.get_id().ok().as_deref() == Some(id) {
                return Ok(dev);
            }
        }
    }
    Err(format!("找不到 capture 裝置 id={id}"))
}

/// System loopback 初始化：render 裝置 + Direction::Capture + EventsShared
/// → wasapi 內部自動帶 AUDCLNT_STREAMFLAGS_LOOPBACK（見 api.rs initialize_client）。
fn init_loopback(ac: &mut wasapi::AudioClient, fmt: &wasapi::WaveFormat) -> Result<(), String> {
    let (_def_time, min_time) = ac.get_device_period().map_err(|e| e.to_string())?;
    let mode = wasapi::StreamMode::EventsShared {
        autoconvert: true,
        buffer_duration_hns: min_time,
    };
    ac.initialize_client(fmt, &wasapi::Direction::Capture, &mode)
        .map_err(|e| e.to_string())
}

/// 依 source 取得 (IAudioClient, WaveFormat)。在已 COM-init 的執行緒內呼叫。
fn open_client(source: &AudioSource) -> Result<(wasapi::AudioClient, wasapi::WaveFormat), String> {
    match source {
        AudioSource::System => {
            let dev = wasapi::get_default_device(&wasapi::Direction::Render)
                .map_err(|e| e.to_string())?;
            let mut ac = dev.get_iaudioclient().map_err(|e| e.to_string())?;
            let fmt = ac.get_mixformat().map_err(|e| e.to_string())?;
            init_loopback(&mut ac, &fmt)?;
            Ok((ac, fmt))
        }
        AudioSource::InputDevice { id } => {
            let dev = find_capture_device_by_id(id)?;
            let mut ac = dev.get_iaudioclient().map_err(|e| e.to_string())?;
            let fmt = ac.get_mixformat().map_err(|e| e.to_string())?;
            // 真·收音：Direction::Capture 一般模式（無 loopback flag）。
            let (_def_time, min_time) = ac.get_device_period().map_err(|e| e.to_string())?;
            let mode = wasapi::StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: min_time,
            };
            ac.initialize_client(&fmt, &wasapi::Direction::Capture, &mode)
                .map_err(|e| e.to_string())?;
            Ok((ac, fmt))
        }
        AudioSource::Process { pid } => {
            // 虛擬裝置不可 get_mixformat → 自帶固定格式 48k stereo f32。
            let mut ac = wasapi::AudioClient::new_application_loopback_client(*pid, true)
                .map_err(|e| e.to_string())?;
            let fmt = wasapi::WaveFormat::new(32, 32, &wasapi::SampleType::Float, 48000, 2, None);
            let mode = wasapi::StreamMode::EventsShared {
                autoconvert: true,
                buffer_duration_hns: 0,
            };
            ac.initialize_client(&fmt, &wasapi::Direction::Capture, &mode)
                .map_err(|e| e.to_string())?;
            Ok((ac, fmt))
        }
    }
}

fn capture_loop(
    source: AudioSource,
    tx: Sender<Vec<f32>>,
    stop: &AtomicBool,
) -> Result<(), String> {
    // COM (MTA)：此執行緒專用，與 Manager / tokio 完全隔離。
    let _ = wasapi::initialize_mta();

    // 1-3. 依 source 取得 (IAudioClient, WaveFormat)，內含裝置選取與 initialize_client。
    let (audio_client, format) = open_client(&source)?;
    let channels = format.get_nchannels() as usize;
    let src_rate = format.get_samplespersec();

    // 4. event handle + capture client，start_stream。
    let h_event = audio_client.set_get_eventhandle().map_err(|e| e.to_string())?;
    let capture_client = audio_client.get_audiocaptureclient().map_err(|e| e.to_string())?;
    audio_client.start_stream().map_err(|e| e.to_string())?;

    // 5. 主迴圈：有上限 event 等待 + 每圈重查 stop（整合契約：≤~200ms 內必退）。
    let mut deque: VecDeque<u8> = VecDeque::new();
    while !stop.load(Ordering::SeqCst) {
        // 逾時/錯誤 → continue 回頭查 stop（切勿無限阻塞）。
        if h_event.wait_for_event(200).is_err() {
            continue;
        }
        // 排空目前所有可用封包到 deque。
        loop {
            match capture_client.get_next_packet_size() {
                Ok(Some(n)) if n > 0 => {
                    if capture_client.read_from_device_to_deque(&mut deque).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
        if deque.is_empty() {
            continue;
        }
        // bytes → 交錯 f32 → 單聲道 → 16k（重用既有純函式）。
        let raw: Vec<u8> = deque.drain(..).collect();
        let interleaved = bytes_to_f32(&raw, &format);
        if interleaved.is_empty() {
            continue;
        }
        let mono = downmix(&interleaved, channels);
        let resampled = resample_to_16k(&mono, src_rate);
        // 非空才送；send 失敗＝consumer 已 drop Receiver → 立即退出（不阻塞、不碰 Manager）。
        if !resampled.is_empty() && tx.send(resampled).is_err() {
            break;
        }
    }

    // 6. 收尾。
    let _ = audio_client.stop_stream();
    Ok(())
}

/// 依混音格式把 raw bytes 解成交錯 f32（先支援 32-bit IEEE float mix format；其餘格式回報待補回空）。
pub fn bytes_to_f32(raw: &[u8], fmt: &wasapi::WaveFormat) -> Vec<f32> {
    let bits = fmt.get_bitspersample();
    let is_float = matches!(fmt.get_subformat(), Ok(wasapi::SampleType::Float));
    if is_float && bits == 32 {
        raw.chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    } else {
        // 16-bit PCM 等其餘格式待補；目前回空避免送出垃圾資料。
        eprintln!("[loopback] 暫不支援的 mix format（bits={bits}, float={is_float}）；待補");
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn downmix_stereo_avg() {
        let m = downmix(&[0.0, 1.0, 1.0, 3.0], 2);
        assert_eq!(m, vec![0.5, 2.0]);
    }
    #[test]
    fn downmix_mono_passthrough() {
        assert_eq!(downmix(&[0.5, -0.5], 1), vec![0.5, -0.5]);
    }
}

#[cfg(test)]
mod resample_tests {
    use super::*;
    #[test]
    fn passthrough_16k() {
        let s = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_to_16k(&s, 16000), s);
    }
    #[test]
    fn downsamples_48k_to_16k_len() {
        let s = vec![0.0f32; 4800];
        let out = resample_to_16k(&s, 48000);
        assert!((out.len() as i64 - 1600).abs() <= 130, "len={}", out.len());
    }
    #[test]
    fn linear_decimate_halves() {
        let out = linear_decimate(&vec![0.0f32; 3200], 32000);
        assert!((out.len() as i64 - 1600).abs() <= 2);
    }
}

#[cfg(test)]
mod vad_tests {
    use super::*;
    #[test]
    fn silence_below_thresh() {
        assert!(!tail_has_speech(&vec![0.0f32; 16000], 1.0, 0.01));
    }
    #[test]
    fn loud_above_thresh() {
        assert!(tail_has_speech(&vec![0.2f32; 16000], 1.0, 0.01));
    }
}

#[cfg(test)]
mod bytes_tests {
    use super::*;
    #[test]
    fn bytes_to_f32_roundtrip_float32() {
        // WaveFormat::new 不需 COM，可在測試中建構。
        let fmt = wasapi::WaveFormat::new(32, 32, &wasapi::SampleType::Float, 48000, 2, None);
        let samples = [0.25f32, -0.5f32, 1.0f32, 0.0f32];
        let mut raw = Vec::new();
        for s in &samples {
            raw.extend_from_slice(&s.to_le_bytes());
        }
        assert_eq!(bytes_to_f32(&raw, &fmt), samples.to_vec());
    }
}
