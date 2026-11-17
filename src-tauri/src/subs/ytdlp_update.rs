//! yt-dlp 更新檢查：比對本機 yt-dlp.exe 與 GitHub 最新 release tag。
use serde::Serialize;
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

const LATEST_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YtdlpUpdateInfo {
    pub local: String,
    pub latest: String,
    pub outdated: bool,
}

/// 從 redirect Location 取出 release tag（`/releases/tag/<tag>` 之後到 `/`、`?`、`#`）。
pub fn tag_from_location(loc: &str) -> Option<String> {
    const MARK: &str = "/releases/tag/";
    let rest = &loc[loc.find(MARK)? + MARK.len()..];
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let tag = &rest[..end];
    (!tag.is_empty()).then(|| tag.to_string())
}

pub fn parse_version(s: &str) -> Option<Vec<u64>> {
    if s.is_empty() {
        return None;
    }
    s.split('.')
        .map(|p| {
            if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) {
                p.parse::<u64>().ok()
            } else {
                None
            }
        })
        .collect()
}

pub fn is_outdated(local: &str, latest: &str) -> Option<bool> {
    let l = parse_version(local)?;
    let n = parse_version(latest)?;
    Some(n > l)
}

fn no_window(cmd: &mut Command) {
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
}

/// 執行 `exe --version --no-update`，回傳 stdout 第一行（須可被 parse_version 解析）。
pub async fn run_version(exe: &Path) -> Result<String, String> {
    let mut cmd = Command::new(exe);
    cmd.args(["--version", "--no-update"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    no_window(&mut cmd);
    let out = tokio::time::timeout(Duration::from_secs(15), cmd.output())
        .await
        .map_err(|_| "yt-dlp --version 逾時".to_string())?
        .map_err(|e| format!("無法執行 yt-dlp: {e}"))?;
    if !out.status.success() {
        return Err(format!("yt-dlp --version 失敗: {}", out.status));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().next().unwrap_or("").trim().to_string();
    if parse_version(&line).is_none() {
        return Err(format!("無法解析 yt-dlp 版本: {line:?}"));
    }
    Ok(line)
}

pub fn cleanup_leftovers(subs_dir: &Path) {
    for n in ["yt-dlp.old.exe", "yt-dlp.new.exe", "yt-dlp.new.exe.part"] {
        let _ = std::fs::remove_file(subs_dir.join(n));
    }
}

/// 安全換檔：`yt-dlp.new.exe` → `yt-dlp.exe`，舊檔先改名為 `yt-dlp.old.exe`（Windows 允許改名執行中的 exe）。
pub async fn swap_in_new(subs_dir: &Path) -> Result<(), String> {
    let exe = subs_dir.join("yt-dlp.exe");
    let old = subs_dir.join("yt-dlp.old.exe");
    let new = subs_dir.join("yt-dlp.new.exe");
    if old.exists() {
        let _ = std::fs::remove_file(&old);
    }
    if let Err(e) = std::fs::rename(&exe, &old) {
        let _ = std::fs::remove_file(&new);
        return Err(e.to_string());
    }
    let mut last = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        match std::fs::rename(&new, &exe) {
            Ok(()) => {
                let _ = std::fs::remove_file(&old);
                return Ok(());
            }
            Err(e) => last = e.to_string(),
        }
    }
    let _ = std::fs::rename(&old, &exe);
    Err(last)
}

/// 驗證 `yt-dlp.new.exe` 可執行後換檔，回傳新版本字串。
pub async fn install_downloaded(subs_dir: &Path) -> Result<String, String> {
    let new = subs_dir.join("yt-dlp.new.exe");
    let ver = match run_version(&new).await {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_file(&new);
            return Err(format!("更新失敗：下載的檔案無法執行（{e}）"));
        }
    };
    swap_in_new(subs_dir)
        .await
        .map_err(|e| format!("更新失敗：無法替換 yt-dlp.exe（{e}）"))?;
    Ok(ver)
}

#[tauri::command]
pub async fn update_ytdlp(app: tauri::AppHandle) -> Result<String, String> {
    use super::download::{download_verify, Asset, YTDLP};
    let subs = crate::data_dir(&app)?.join("subs");
    let asset = Asset {
        url: YTDLP.url,
        sha256: YTDLP.sha256,
        filename: "yt-dlp.new.exe",
        is_zip: false,
    };
    download_verify(&asset, &subs, super::session::prog_emit(app.clone(), "ytdlp-update"))
        .await
        .map_err(|e| format!("更新失敗：下載失敗（{e}）"))?;
    install_downloaded(&subs).await
}

#[tauri::command]
pub async fn check_ytdlp_update(app: tauri::AppHandle) -> Result<YtdlpUpdateInfo, String> {
    let subs_dir = crate::data_dir(&app)?.join("subs");
    cleanup_leftovers(&subs_dir);
    let exe = subs_dir.join("yt-dlp.exe");
    if !exe.exists() {
        return Err("yt-dlp.exe 不存在".into());
    }
    let local = run_version(&exe).await?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(LATEST_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_redirection() {
        return Err(format!("非預期的狀態碼: {}", resp.status()));
    }
    let loc = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or("缺少 Location header")?;
    let latest = tag_from_location(loc).ok_or("無法從 Location 取得 tag")?;
    let outdated = is_outdated(&local, &latest).ok_or("版本字串無法解析")?;
    Ok(YtdlpUpdateInfo { local, latest, outdated })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_parsing() {
        let b = "https://github.com/yt-dlp/yt-dlp/releases/tag/2026.10.26";
        assert_eq!(tag_from_location(b).as_deref(), Some("2026.10.26"));
        for s in ["?x=1", "#a", "/"] {
            assert_eq!(tag_from_location(&format!("{b}{s}")).as_deref(), Some("2026.10.26"));
        }
        assert_eq!(tag_from_location("https://github.com/yt-dlp/yt-dlp/releases"), None);
        assert_eq!(tag_from_location(""), None);
    }

    #[test]
    fn version_parsing() {
        assert_eq!(parse_version("2026.10.26"), Some(vec![2026, 10, 26]));
        assert_eq!(parse_version("2026.10.26.1"), Some(vec![2026, 10, 26, 1]));
        for s in ["2026.10.x", "", "v2026.10.26"] {
            assert_eq!(parse_version(s), None);
        }
    }

    #[test]
    fn outdated_cmp() {
        assert_eq!(is_outdated("2026.06.09", "2026.10.26"), Some(true));
        assert_eq!(is_outdated("2026.10.26", "2026.10.26"), Some(false));
        assert_eq!(is_outdated("2026.10.27", "2026.10.26"), Some(false));
        assert_eq!(is_outdated("2026.10.26", "2026.10.26.1"), Some(true));
        assert_eq!(is_outdated("2026.10.26.1", "2026.10.26"), Some(false));
        assert_eq!(is_outdated("x", "2026.10.26"), None);
        assert_eq!(is_outdated("2026.10.26", "x"), None);
    }

    #[test]
    fn cleanup_removes_leftovers_only() {
        let d = std::env::temp_dir().join(format!("ytu-cleanup-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for n in ["yt-dlp.old.exe", "yt-dlp.new.exe", "yt-dlp.new.exe.part", "yt-dlp.exe"] {
            std::fs::write(d.join(n), b"x").unwrap();
        }
        cleanup_leftovers(&d);
        assert!(!d.join("yt-dlp.old.exe").exists());
        assert!(!d.join("yt-dlp.new.exe").exists());
        assert!(!d.join("yt-dlp.new.exe.part").exists());
        assert!(d.join("yt-dlp.exe").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    fn tdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ytu-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn swap_ok() {
        let d = tdir("swapok");
        std::fs::write(d.join("yt-dlp.exe"), b"OLD").unwrap();
        std::fs::write(d.join("yt-dlp.new.exe"), b"NEW").unwrap();
        swap_in_new(&d).await.unwrap();
        assert_eq!(std::fs::read(d.join("yt-dlp.exe")).unwrap(), b"NEW");
        assert!(!d.join("yt-dlp.new.exe").exists());
        assert!(!d.join("yt-dlp.old.exe").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn swap_ok_when_old_leftover_exists() {
        let d = tdir("swapstale");
        std::fs::write(d.join("yt-dlp.exe"), b"OLD").unwrap();
        std::fs::write(d.join("yt-dlp.old.exe"), b"STALE").unwrap();
        std::fs::write(d.join("yt-dlp.new.exe"), b"NEW").unwrap();
        swap_in_new(&d).await.unwrap();
        assert_eq!(std::fs::read(d.join("yt-dlp.exe")).unwrap(), b"NEW");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn swap_restores_when_new_missing() {
        let d = tdir("swaprestore");
        std::fs::write(d.join("yt-dlp.exe"), b"OLD").unwrap();
        assert!(swap_in_new(&d).await.is_err());
        assert_eq!(std::fs::read(d.join("yt-dlp.exe")).unwrap(), b"OLD");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[tokio::test]
    async fn install_rejects_garbage() {
        let d = tdir("garbage");
        std::fs::write(d.join("yt-dlp.exe"), b"OLD").unwrap();
        std::fs::write(d.join("yt-dlp.new.exe"), b"<html>404</html>").unwrap();
        let e = install_downloaded(&d).await.unwrap_err();
        assert!(e.contains("下載的檔案無法執行"), "{e}");
        assert!(!d.join("yt-dlp.new.exe").exists());
        assert_eq!(std::fs::read(d.join("yt-dlp.exe")).unwrap(), b"OLD");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn swap_while_running() {
        let d = tdir("running");
        std::fs::copy(r"C:\Windows\System32\PING.EXE", d.join("yt-dlp.exe")).unwrap();
        let mut child = std::process::Command::new(d.join("yt-dlp.exe"))
            .args(["-n", "10", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::fs::write(d.join("yt-dlp.new.exe"), b"NEW").unwrap();
        let r = swap_in_new(&d).await;
        let content = std::fs::read(d.join("yt-dlp.exe"));
        let _ = child.kill();
        let _ = child.wait();
        r.unwrap();
        assert_eq!(content.unwrap(), b"NEW");
        let _ = std::fs::remove_dir_all(&d);
    }
}
