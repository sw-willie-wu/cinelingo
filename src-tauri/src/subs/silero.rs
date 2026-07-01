//! Silero VAD helpers: frame splitting, hysteresis labelling, and VadGate integration.

use voice_activity_detector::VoiceActivityDetector;

pub const FRAME: usize = 512;
pub const SR: i64 = 16000;

/// 從 buf 前端 drain 出所有完整 frame-長框，餘數留 buf。純（除 mutate buf）。
pub fn split_frames(buf: &mut Vec<f32>, frame: usize) -> Vec<Vec<f32>> {
    debug_assert!(frame > 0, "frame must be > 0");
    let n = buf.len() / frame;
    if n == 0 { return Vec::new(); }
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        out.push(buf[k * frame..(k + 1) * frame].to_vec());
    }
    buf.drain(0..n * frame);
    out
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LabelState {
    pub in_speech: bool,
    pub onset_run: u32,
}

/// 單框遲滯標籤。speech_thresh/neg_thresh 雙門檻 + min_speech_frames 防單框抖動。純函式。
pub fn label_frame(
    st: LabelState,
    prob: f32,
    speech_thresh: f32,
    neg_thresh: f32,
    min_speech_frames: u32,
) -> LabelState {
    if prob >= speech_thresh {
        let onset_run = st.onset_run + 1;
        let in_speech = st.in_speech || onset_run >= min_speech_frames.max(1);
        LabelState { in_speech, onset_run }
    } else if prob < neg_thresh {
        LabelState {
            in_speech: false,
            onset_run: 0,
        }
    } else {
        LabelState {
            in_speech: st.in_speech,
            onset_run: 0,
        }
    }
}

pub const BOUNDARY_FLOOR_SEC: f64 = 0.1;

/// vad_min_silence_ms → 句界靜音門檻（秒），加 floor 防 100ms 預設過度切碎。純函式。
pub fn boundary_threshold_sec(vad_min_silence_ms: i64) -> f64 {
    (vad_min_silence_ms as f64 / 1000.0).max(BOUNDARY_FLOOR_SEC)
}

/// UI 靈敏度 (0–1) → silero speech 機率門檻，夾 [0.1,0.9]（預設 0.5→0.5）。純函式。
pub fn vad_threshold_to_prob(threshold: f64) -> f32 {
    threshold.clamp(0.1, 0.9) as f32
}

/// 行程內 Silero VAD gate。吃 16k mono f32 chunks，回 per-tick 語音 bool。
/// 不持有絕對時間——時間基準在 run_loop（captured_sec/last_speech_sec）。
/// 整軌批次口（Path A）留下個 epic，本 epic 只串流。
pub struct VadGate {
    vad: VoiceActivityDetector,
    buf: Vec<f32>,
    label: LabelState,
    speech_thresh: f32,
    neg_thresh: f32,
    min_speech_frames: u32,
}

impl VadGate {
    pub fn new(speech_thresh: f32, neg_thresh: f32, min_speech_frames: u32) -> Result<Self, String> {
        let vad = VoiceActivityDetector::builder()
            .sample_rate(SR)
            .chunk_size(FRAME)
            .build()
            .map_err(|e| format!("VAD build: {e}"))?;
        Ok(Self {
            vad,
            buf: Vec::new(),
            label: LabelState::default(),
            speech_thresh,
            neg_thresh,
            min_speech_frames,
        })
    }

    /// 餵本批新樣本，回本批是否含任一語音框。
    pub fn push(&mut self, samples: &[f32]) -> bool {
        self.buf.extend_from_slice(samples);
        let frames = split_frames(&mut self.buf, FRAME);
        let mut saw_speech = false;
        for f in frames {
            let prob: f32 = self.vad.predict(f);
            self.label = label_frame(
                self.label,
                prob,
                self.speech_thresh,
                self.neg_thresh,
                self.min_speech_frames,
            );
            if self.label.in_speech {
                saw_speech = true;
            }
        }
        saw_speech
    }

    /// 清 NN 狀態 + buf + label（CC 重開 / session 換源用）。
    pub fn reset(&mut self) {
        self.vad.reset();
        self.buf.clear();
        self.label = LabelState::default();
    }

    /// 動態更新雙門檻（不碰 NN 狀態 / buf / label）。run_loop 每輪依 UI 靈敏度呼叫。
    pub fn set_thresholds(&mut self, speech_thresh: f32, neg_thresh: f32) {
        self.speech_thresh = speech_thresh;
        self.neg_thresh = neg_thresh;
    }
}

#[cfg(test)]
fn set_ort_dylib_for_test() {
    let dll = concat!(env!("CARGO_MANIFEST_DIR"), "/lib/onnxruntime.dll");
    std::env::set_var("ORT_DYLIB_PATH", dll);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frames_drains_complete_and_keeps_remainder() {
        let mut buf: Vec<f32> = (0..1100).map(|i| i as f32).collect(); // 1100 = 2*512 + 76
        let frames = split_frames(&mut buf, 512);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].len(), 512);
        assert_eq!(frames[1].len(), 512);
        assert_eq!(frames[0][0], 0.0);
        assert_eq!(frames[1][0], 512.0);
        assert_eq!(buf.len(), 76, "餘數留在 buf");
        assert_eq!(buf[0], 1024.0);
    }

    #[test]
    fn split_frames_under_one_frame_keeps_all() {
        let mut buf: Vec<f32> = vec![1.0; 300];
        let frames = split_frames(&mut buf, 512);
        assert!(frames.is_empty());
        assert_eq!(buf.len(), 300);
    }

    #[test]
    fn split_frames_exact_multiple_empties_buf() {
        let mut buf: Vec<f32> = vec![1.0; 1024];
        let frames = split_frames(&mut buf, 512);
        assert_eq!(frames.len(), 2);
        assert!(buf.is_empty());
    }

    #[test]
    fn label_frame_onset_needs_min_speech_frames() {
        let st = LabelState::default();
        let s1 = label_frame(st, 0.9, 0.5, 0.35, 2);
        assert!(!s1.in_speech);
        assert_eq!(s1.onset_run, 1);
        let s2 = label_frame(s1, 0.9, 0.5, 0.35, 2);
        assert!(s2.in_speech);
    }

    #[test]
    fn label_frame_offset_immediate_below_neg() {
        let st = LabelState { in_speech: true, onset_run: 5 };
        let s = label_frame(st, 0.2, 0.5, 0.35, 2);
        assert!(!s.in_speech, "低於 neg 門檻立即退語音");
        assert_eq!(s.onset_run, 0);
    }

    #[test]
    fn label_frame_hysteresis_band_holds_state() {
        let on = label_frame(LabelState { in_speech: true, onset_run: 3 }, 0.4, 0.5, 0.35, 2);
        assert!(on.in_speech, "帶內維持語音");
        let off = label_frame(LabelState::default(), 0.4, 0.5, 0.35, 2);
        assert!(!off.in_speech, "帶內維持非語音");
        assert_eq!(off.onset_run, 0, "未達 speech 門檻 run 歸零");
    }

    #[test]
    #[ignore = "需 onnxruntime.dll；cargo test -- --ignored"]
    fn vadgate_push_silence_no_speech() {
        set_ort_dylib_for_test();
        let mut g = VadGate::new(0.5, 0.35, 2).expect("new gate");
        let speech = g.push(&vec![0.0_f32; 1024]); // 兩框靜音
        assert!(!speech, "靜音不該判語音");
    }

    #[test]
    #[ignore = "需 onnxruntime.dll；cargo test -- --ignored"]
    fn vadgate_push_under_frame_buffers() {
        set_ort_dylib_for_test();
        let mut g = VadGate::new(0.5, 0.35, 2).expect("new gate");
        assert!(!g.push(&vec![0.0_f32; 300])); // 不足一框 → 不推論、回 false、留 buf
    }

    #[test]
    #[ignore = "需 onnxruntime.dll；cargo test -- --ignored"]
    fn vadgate_reset_clears_state() {
        set_ort_dylib_for_test();
        let mut g = VadGate::new(0.5, 0.35, 2).expect("new gate");
        let _ = g.push(&vec![0.0_f32; 1024]);
        g.reset();
        assert!(!g.push(&vec![0.0_f32; 100])); // reset 後 buf 空、不足框 → false
    }

    #[test]
    #[ignore = "需 onnxruntime.dll；cargo test -- --ignored"]
    fn vadgate_set_thresholds_smoke() {
        set_ort_dylib_for_test();
        let mut g = VadGate::new(0.5, 0.35, 2).expect("new gate");
        g.set_thresholds(0.8, 0.6);   // 改門檻不應 panic、不碰狀態
        assert!(!g.push(&vec![0.0_f32; 1024])); // 靜音仍非語音
    }

    #[test]
    fn boundary_threshold_has_floor() {
        assert!((boundary_threshold_sec(700) - 0.7).abs() < 1e-9);
        assert!((boundary_threshold_sec(200) - 0.2).abs() < 1e-9);          // 高於 floor 直通
        assert!((boundary_threshold_sec(50) - BOUNDARY_FLOOR_SEC).abs() < 1e-9);  // 低於 floor 夾住
        assert!((boundary_threshold_sec(0) - BOUNDARY_FLOOR_SEC).abs() < 1e-9);
    }

    #[test]
    fn vad_threshold_maps_and_clamps() {
        assert!((vad_threshold_to_prob(0.5) - 0.5).abs() < 1e-6);
        assert!((vad_threshold_to_prob(0.0) - 0.1).abs() < 1e-6); // 夾下限
        assert!((vad_threshold_to_prob(1.0) - 0.9).abs() < 1e-6); // 夾上限
    }
}
