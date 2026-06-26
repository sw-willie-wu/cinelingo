use std::cmp::Ordering;
use std::path::Path;

/// 可播放副檔名（與前端 drop.ts 對齊；此處為後端權威來源）。
const PLAYABLE: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "avi", "mov", "flv", "wmv", "ts", "m2ts",
    "mpg", "mpeg", "h264", "h265", "hevc", "3gp", "rmvb",
    "mp3", "flac", "aac", "m4a", "ogg", "opus", "wav", "wma",
];

fn is_playable_path(p: &str) -> bool {
    Path::new(p)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| PLAYABLE.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn collect_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(c) = it.peek().copied() {
        if c.is_ascii_digit() {
            s.push(c);
            it.next();
        } else {
            break;
        }
    }
    s
}

/// 自然排序：把字串切成數字 / 非數字段逐段比較（ep2 < ep10）。非數字段不分大小寫。
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ca), Some(cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    let na = collect_digits(&mut ai);
                    let nb = collect_digits(&mut bi);
                    // 去前導 0 後比長度再比字典序
                    let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                    let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
                    if la != lb {
                        return la.cmp(&lb);
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}

fn collect_dir_recursive(dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort_by(|a, b| natural_cmp(&a.to_string_lossy(), &b.to_string_lossy()));
    for p in entries {
        if p.is_dir() {
            collect_dir_recursive(&p, out);
        } else if let Some(s) = p.to_str() {
            if is_playable_path(s) {
                out.push(s.to_string());
            }
        }
    }
}

/// 拖放展開：逐路徑 stat；檔→可播則放行；資料夾→遞迴收可播檔（自然排序）。回扁平有序清單。
#[tauri::command]
pub async fn expand_playable_paths(paths: Vec<String>) -> Result<Vec<String>, String> {
    let out = tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        for p in &paths {
            let path = Path::new(p);
            if path.is_dir() {
                collect_dir_recursive(path, &mut out);
            } else if is_playable_path(p) {
                out.push(p.clone());
            }
        }
        out
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(out)
}

/// 檔案是否存在（最近清單失效檢查）。
#[tauri::command]
pub async fn path_exists(path: String) -> Result<bool, String> {
    Ok(tokio::fs::try_exists(&path).await.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_cmp_orders_numbers_humanly() {
        let mut v = vec!["ep10.mkv".to_string(), "ep2.mkv".to_string(), "ep1.mkv".to_string()];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, vec!["ep1.mkv", "ep2.mkv", "ep10.mkv"]);
    }

    #[test]
    fn is_playable_checks_ext_case_insensitive() {
        assert!(is_playable_path("C:/a/CLIP.MP4"));
        assert!(is_playable_path("x.mkv"));
        assert!(!is_playable_path("x.txt"));
        assert!(!is_playable_path("noext"));
    }
}
