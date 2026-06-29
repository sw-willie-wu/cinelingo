use super::cue::{derive_cue_id, Cue};

#[derive(Debug, Clone)]
pub struct RawSeg {
    pub start_sec: f64,
    pub end_sec: f64,
    pub text: String,
}

/// 視窗相對段落 → 絕對 Cue（id 由絕對 startMs 決定）。跳過空白。
/// 去重由呼叫端（covered/emitted）處理。
pub fn merge_segments(session_id: &str, lang: Option<&str>, window_start: f64, segs: &[RawSeg]) -> Vec<Cue> {
    let mut out = Vec::new();
    for s in segs {
        let start = window_start + s.start_sec;
        let end = window_start + s.end_sec;
        let text = s.text.trim().to_string();
        if text.is_empty() || end <= start {
            continue;
        }
        out.push(Cue {
            id: derive_cue_id((start * 1000.0).round() as i64),
            session_id: session_id.to_string(),
            start_sec: start,
            end_sec: end,
            source_text: text,
            lang: lang.map(|s| s.to_string()),
            status: "final".into(),
            ..Default::default()
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(a: f64, b: f64, t: &str) -> RawSeg {
        RawSeg { start_sec: a, end_sec: b, text: t.into() }
    }

    #[test]
    fn offsets() {
        let c = merge_segments("s", Some("ja"), 10.0, &[seg(0.0, 2.0, "hi")]);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].start_sec, 10.0);
        assert_eq!(c[0].end_sec, 12.0);
        assert_eq!(c[0].lang.as_deref(), Some("ja"));
    }

    #[test]
    fn id_abs() {
        let c = merge_segments("s", None, 10.0, &[seg(0.5, 2.0, "x")]);
        assert_eq!(c[0].id, "10500");
    }

    #[test]
    fn skips_empty() {
        let c = merge_segments("s", None, 0.0, &[seg(0.0, 1.0, "  ")]);
        assert!(c.is_empty());
    }
}
