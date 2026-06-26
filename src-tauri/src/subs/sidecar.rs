use std::path::Path;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSub {
    pub name: String,
    pub path: String,
}

/// 掃影片父資料夾，回符合 `is_sidecar_sub` 的字幕（唯讀；無父目錄/讀目錄失敗 → 空 Vec）。
pub fn scan_sidecar_dir(video_path: &str) -> Vec<SidecarSub> {
    let p = Path::new(video_path);
    let Some(dir) = p.parent() else {
        return Vec::new();
    };
    let video_name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let name = e.file_name();
        let Some(n) = name.to_str() else { continue };
        if is_sidecar_sub(video_name, n) {
            out.push(SidecarSub {
                name: n.to_string(),
                path: e.path().to_string_lossy().to_string(),
            });
        }
    }
    out
}

/// Part A 指令：列出影片同資料夾字幕（唯讀，不阻斷播放）。
#[tauri::command]
pub async fn list_sidecar_subs(video_path: String) -> Result<Vec<SidecarSub>, String> {
    let subs = tauri::async_runtime::spawn_blocking(move || scan_sidecar_dir(&video_path))
        .await
        .map_err(|e| e.to_string())?;
    Ok(subs)
}

/// `candidate` 是否為 `video_filename` 的同資料夾字幕：副檔名 srt/vtt（不分大小寫），
/// 且檔名以「影片基底名 + '.'」開頭（不分大小寫；Windows 檔案系統大小寫不敏感）。
/// `movies.srt` 不會誤判（要求基底名後緊接 '.'）。
pub fn is_sidecar_sub(video_filename: &str, candidate: &str) -> bool {
    let cand = candidate.to_lowercase();
    if !(cand.ends_with(".srt") || cand.ends_with(".vtt")) {
        return false;
    }
    let video_lc = video_filename.to_lowercase();
    let stem = match video_lc.rsplit_once('.') {
        Some((s, _)) => s,
        None => video_lc.as_str(),
    };
    if stem.is_empty() {
        return false;
    }
    cand.starts_with(&format!("{stem}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_same_stem_and_lang_suffix() {
        assert!(is_sidecar_sub("Movie.mkv", "movie.srt"));
        assert!(is_sidecar_sub("Movie.mkv", "Movie.zh-TW.srt"));
        assert!(is_sidecar_sub("Movie.mkv", "movie.en.vtt"));
    }

    #[test]
    fn case_insensitive_stem() {
        assert!(is_sidecar_sub("Movie.mkv", "MOVIE.SRT"));
        assert!(is_sidecar_sub("movie.MKV", "Movie.ja.srt"));
    }

    #[test]
    fn rejects_prefix_collision() {
        assert!(!is_sidecar_sub("Movie.mkv", "movies.srt"));
    }

    #[test]
    fn rejects_non_sub_ext() {
        assert!(!is_sidecar_sub("Movie.mkv", "movie.txt"));
        assert!(!is_sidecar_sub("Movie.mkv", "movie.ass"));
    }

    #[test]
    fn excludes_video_itself() {
        assert!(!is_sidecar_sub("Movie.mkv", "Movie.mkv"));
    }

    #[test]
    fn empty_stem_is_rejected() {
        assert!(!is_sidecar_sub(".mkv", ".srt"));
    }

    #[test]
    fn dotless_video_filename_matches() {
        assert!(is_sidecar_sub("Movie", "Movie.srt"));
    }

    #[test]
    fn scan_lists_matching_subs_only() {
        let dir = std::env::temp_dir().join(format!("lmpv-sidecar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let video = dir.join("Movie.mkv");
        std::fs::write(&video, b"x").unwrap();
        std::fs::write(dir.join("Movie.srt"), b"x").unwrap();
        std::fs::write(dir.join("Movie.zh-TW.srt"), b"x").unwrap();
        std::fs::write(dir.join("movies.srt"), b"x").unwrap(); // 前綴誤判 → 不列
        std::fs::write(dir.join("notes.txt"), b"x").unwrap();  // 非字幕 → 不列
        let mut names: Vec<String> =
            scan_sidecar_dir(video.to_str().unwrap()).into_iter().map(|s| s.name).collect();
        names.sort();
        assert_eq!(names, vec!["Movie.srt".to_string(), "Movie.zh-TW.srt".to_string()]);
        std::fs::remove_dir_all(&dir).ok();
    }
}
