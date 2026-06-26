pub const WINDOW_SEC: f64 = 28.0;
pub const OVERLAP_SEC: f64 = 2.0;
/// 對齊退路門檻：窗尾退到語音段前的靜音縫後，若新內容不足此長度就改硬切，避免一堆超短窗。
pub const MIN_WINDOW_SEC: f64 = 12.0;
pub type Intervals = Vec<(f64, f64)>;
const EPS: f64 = 0.05;

/// 併入 [s,e] 並合併重疊/相鄰（容差 EPS），維持排序。
pub fn add_interval(mut v: Intervals, s: f64, e: f64) -> Intervals {
    if e <= s {
        return v;
    }
    v.push((s, e));
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Intervals = Vec::new();
    for (s, e) in v {
        if let Some(last) = out.last_mut() {
            if s <= last.1 + EPS {
                if e > last.1 {
                    last.1 = e;
                }
                continue;
            }
        }
        out.push((s, e));
    }
    out
}

/// [s,e] 是否完全落在某覆蓋區間內。
pub fn fully_covers(v: &Intervals, s: f64, e: f64) -> bool {
    v.iter().any(|(a, b)| *a <= s + EPS && e <= *b + EPS)
}

/// from 起、duration 內第一個未覆蓋點；全覆蓋回 None。（v 須排序合併）
pub fn first_uncovered(v: &Intervals, from: f64, duration: f64) -> Option<f64> {
    if from >= duration {
        return None;
    }
    let mut cur = from;
    for (a, b) in v {
        if *b <= cur + EPS {
            continue;
        }
        if *a > cur + EPS {
            return Some(cur);
        }
        cur = *b;
        if cur >= duration {
            return None;
        }
    }
    if cur < duration {
        Some(cur)
    } else {
        None
    }
}

/// 把硬窗尾 max_end 對齊到「不切穿語音段」的位置。
/// speech 已排序、不重疊、為秒（VAD 工具天然輸出）；故跨越 max_end 的語音段至多一個。
/// point 為本窗未覆蓋起點；max_end = min(point + WINDOW_SEC, duration)（呼叫端保證 > point）。
/// 回傳 end 永遠 > point：回 `a`（> point + MIN_WINDOW_SEC）或 `max_end`（> point）。
pub fn aligned_window_end(speech: &Intervals, point: f64, max_end: f64) -> f64 {
    for (a, b) in speech {
        // max_end 嚴格落在 (a, b) 內（含 EPS 容差）＝切穿這段語音。
        if *a + EPS < max_end && max_end + EPS < *b {
            // 退到該段之前的靜音縫（段首 a）；窗會太短則硬切（連續長語音段退路）。
            if *a > point + MIN_WINDOW_SEC {
                return *a;
            }
            return max_end;
        }
    }
    max_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges() {
        let v = add_interval(add_interval(vec![], 0.0, 10.0), 9.0, 20.0);
        assert_eq!(v, vec![(0.0, 20.0)]);
    }

    #[test]
    fn keeps_gap() {
        let v = add_interval(add_interval(vec![], 0.0, 10.0), 15.0, 20.0);
        assert_eq!(v, vec![(0.0, 10.0), (15.0, 20.0)]);
    }

    #[test]
    fn fully() {
        let v = vec![(0.0, 10.0)];
        assert!(fully_covers(&v, 2.0, 8.0));
        assert!(!fully_covers(&v, 8.0, 12.0));
    }

    #[test]
    fn first_unc_seq() {
        assert_eq!(first_uncovered(&vec![], 0.0, 100.0), Some(0.0));
        assert_eq!(first_uncovered(&vec![(0.0, 30.0)], 0.0, 100.0), Some(30.0));
    }

    #[test]
    fn first_unc_gap() {
        assert_eq!(first_uncovered(&vec![(0.0, 30.0), (60.0, 90.0)], 0.0, 100.0), Some(30.0));
        assert_eq!(first_uncovered(&vec![(0.0, 30.0), (60.0, 90.0)], 65.0, 100.0), Some(90.0));
    }

    #[test]
    fn first_unc_done() {
        assert_eq!(first_uncovered(&vec![(0.0, 100.0)], 0.0, 100.0), None);
    }

    #[test]
    fn align_straddle_returns_segment_start() {
        // 語音段 (26,33) 跨越 max_end=28 → 退到 26（26 > point0+MIN_WINDOW_SEC）
        let sp = vec![(1.0, 4.0), (26.0, 33.0)];
        assert_eq!(aligned_window_end(&sp, 0.0, 28.0), 26.0);
    }

    #[test]
    fn align_no_straddle_returns_max_end() {
        // max_end=28 落在靜音、無段跨越 → max_end
        let sp = vec![(1.0, 4.0), (30.0, 35.0)];
        assert_eq!(aligned_window_end(&sp, 0.0, 28.0), 28.0);
    }

    #[test]
    fn align_long_segment_hard_cuts() {
        // 跨越段 (5,80)，a=5 不 > point0+MIN_WINDOW_SEC(12) → 硬切 max_end
        let sp = vec![(5.0, 80.0)];
        assert_eq!(aligned_window_end(&sp, 0.0, 28.0), 28.0);
    }

    #[test]
    fn align_empty_speech_returns_max_end() {
        assert_eq!(aligned_window_end(&vec![], 0.0, 28.0), 28.0);
    }

    #[test]
    fn align_max_end_at_segment_end_not_straddle() {
        // max_end == 段尾 b → 不算跨界（無切）→ max_end
        let sp = vec![(10.0, 28.0)];
        assert_eq!(aligned_window_end(&sp, 0.0, 28.0), 28.0);
    }

    #[test]
    fn align_max_end_at_segment_start_not_straddle() {
        // max_end == 段首 a → 不算跨界 → max_end（=28）
        let sp = vec![(28.0, 40.0)];
        assert_eq!(aligned_window_end(&sp, 0.0, 28.0), 28.0);
    }
}
