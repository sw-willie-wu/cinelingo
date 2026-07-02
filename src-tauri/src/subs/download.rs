use super::hwdetect::Backend;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub struct Asset {
    pub url: &'static str,
    pub sha256: &'static str,
    pub filename: &'static str,
    #[allow(dead_code)] // 保留供日後（zip vs 直接檔）；目前由呼叫端 context 決定是否 unzip
    pub is_zip: bool,
}

/// 使用者可選模型大小（spec §6.2）。host=ggerganov/whisper.cpp（Task 9 probe 確認；ggml-org/whisper.cpp 會 401）。
#[derive(Clone, Copy, Debug)]
pub enum ModelSize {
    Small,
    Medium,
    Turbo,
    LargeV3,
}

pub fn model_from_key(k: &str) -> ModelSize {
    match k {
        "small" => ModelSize::Small,
        "medium" => ModelSize::Medium,
        "large-v3" => ModelSize::LargeV3,
        _ => ModelSize::Turbo, // 預設
    }
}

pub fn model_asset(m: ModelSize) -> Asset {
    match m {
        ModelSize::Small => Asset {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
            filename: "ggml-small.bin",
            is_zip: false,
        }, // 465MB
        ModelSize::Medium => Asset {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
            sha256: "<SKIP>",
            filename: "ggml-medium.bin",
            is_zip: false,
        }, // 1463MB
        ModelSize::Turbo => Asset {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
            sha256: "<SKIP>",
            filename: "ggml-large-v3-turbo.bin",
            is_zip: false,
        }, // 1549MB（預設）
        ModelSize::LargeV3 => Asset {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
            sha256: "<SKIP>",
            filename: "ggml-large-v3.bin",
            is_zip: false,
        }, // 2952MB
    }
}

pub const MODEL_VAD: Asset = Asset {
    url: "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v5.1.2.bin",
    sha256: "29940d98d42b91fbd05ce489f3ecf7c72f0a42f027e4875919a28fb4c04ea2cf",
    filename: "ggml-silero-v5.1.2.bin",
    is_zip: false,
}; // 0.8MB

pub const FFMPEG: Asset = Asset {
    // GitHub GyanD/codexffmpeg essentials（單一靜態 ffmpeg.exe，含常見格式；GitHub CDN 可靠、版本化）。
    // gyan.dev 直連對程式化下載不可靠（0-byte 卡死）→ 改 GitHub。版本已釘(8.1.1) → 發佈前可補真 sha；GPL build 之授權見 spec §7。
    url: "https://github.com/GyanD/codexffmpeg/releases/download/8.1.1/ffmpeg-8.1.1-essentials_build.zip",
    sha256: "<SKIP>",
    filename: "ffmpeg.zip",
    is_zip: true,
};

pub const YTDLP: Asset = Asset {
    // Single exe; use "latest" so the extractor can be refreshed when YouTube breaks it (sha <SKIP> = version-loose).
    url: "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe",
    sha256: "<SKIP>",
    filename: "yt-dlp.exe",
    is_zip: false,
};

/// P3：Vulkan 未託管 → fallback CPU（§12）。sha256 由 Task 9 probe 確認。
pub fn backend_asset(b: Backend) -> Asset {
    match b {
        Backend::Cuda => Asset {
            url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.8.6/whisper-cublas-12.4.0-bin-x64.zip",
            sha256: "63b70c91fe2fd7449865c45f6422ab628439eacc6985d8309c77bfb65cc68a19",
            filename: "whisper-cuda.zip",
            is_zip: true,
        }, // 438.5MB
        _ => Asset {
            url: "https://github.com/ggml-org/whisper.cpp/releases/download/v1.8.6/whisper-bin-x64.zip",
            sha256: "b07ea0b1b4115a38e1a7b07debf581f0b77d999925f8acb8f39d322b0ba0a822",
            filename: "whisper-cpu.zip",
            is_zip: true,
        }, // 3.9MB
    }
}

/// 檢查指定 model key 的 .bin 是否已存在於 `subs_dir`（reuse model_asset 的檔名，路徑釘死）。
pub fn model_downloaded_in(subs_dir: &Path, key: &str) -> bool {
    subs_dir.join(model_asset(model_from_key(key)).filename).exists()
}

/// 各資產約略下載大小（MB）。供 check_engine 估算「約 X GB」。
pub const SIZE_MB_TURBO: u64 = 1549;
pub const SIZE_MB_VAD: u64 = 1; // 0.8MB 進位
pub const SIZE_MB_BACKEND_CUDA: u64 = 439; // 438.5 進位
pub const SIZE_MB_BACKEND_CPU: u64 = 4; // 3.9 進位
pub const SIZE_MB_FFMPEG: u64 = 80; // TODO(impl): 下載一次量測 GyanD 8.1.1 essentials zip 後釘定真值
#[allow(dead_code)]
pub const SIZE_MB_LLAMA_CUDA: u64 = 400;
#[allow(dead_code)]
pub const SIZE_MB_LLAMA_CPU: u64 = 20;
pub const SIZE_MB_TG_4B: u64 = 2490;
pub const SIZE_MB_TG_12B: u64 = 6962;

pub const TRANSLATEGEMMA_4B: Asset = Asset {
    url: "https://huggingface.co/bullerwins/translategemma-4b-it-GGUF/resolve/main/translategemma-4b-it-Q4_K_M.gguf",
    sha256: "<SKIP>",
    filename: "translategemma-4b-it-Q4_K_M.gguf",
    is_zip: false,
};
pub const TRANSLATEGEMMA_12B: Asset = Asset {
    url: "https://huggingface.co/bullerwins/translategemma-12b-it-GGUF/resolve/main/translategemma-12b-it-Q4_K_M.gguf",
    sha256: "<SKIP>",
    filename: "translategemma-12b-it-Q4_K_M.gguf",
    is_zip: false,
};

/// translate-model key → Asset（未知 → 4B）。
pub fn translate_model_from_key(k: &str) -> &'static Asset {
    match k {
        "translate-12b" => &TRANSLATEGEMMA_12B,
        _ => &TRANSLATEGEMMA_4B,
    }
}
pub fn translate_model_asset(k: &str) -> &'static Asset { translate_model_from_key(k) }
pub fn translate_model_size_mb(k: &str) -> u64 {
    match k { "translate-12b" => SIZE_MB_TG_12B, _ => SIZE_MB_TG_4B }
}
pub const TRANSLATE_MODEL_KEYS: [&str; 2] = ["translate-4b", "translate-12b"];

#[allow(dead_code)] // consumed by Task 3+
pub fn llama_backend_asset(b: Backend) -> Asset {
    match b {
        Backend::Cuda => Asset {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b9843/llama-b9843-bin-win-cuda-12.4-x64.zip",
            sha256: "<SKIP>",
            filename: "llama-cuda.zip",
            is_zip: true,
        },
        _ => Asset {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b9843/llama-b9843-bin-win-cpu-x64.zip",
            sha256: "<SKIP>",
            filename: "llama-cpu.zip",
            is_zip: true,
        },
    }
}

/// CUDA runtime DLL（與 llama-cuda binaries 分開發佈，CUDA 必需）。
/// cudart64_12.dll 等不含於 binaries zip → 須另抓並解到同一 llm_dir。
#[allow(dead_code)] // consumed by ensure_llm_assets (CUDA path)
pub fn cudart_asset() -> Asset {
    Asset {
        url: "https://github.com/ggml-org/llama.cpp/releases/download/b9843/cudart-llama-bin-win-cuda-12.4-x64.zip",
        sha256: "<SKIP>",
        filename: "cudart.zip",
        is_zip: true,
    }
}

#[allow(dead_code)] // consumed by Task 3+
pub fn llm_dir(data_root: &Path) -> PathBuf { data_root.join("llm") }
pub fn translate_model_downloaded(llm_dir: &Path, key: &str) -> bool {
    llm_dir.join(translate_model_asset(key).filename).exists()
}
#[allow(dead_code)] // consumed by Task 3+
pub fn llama_server_downloaded(llm_dir: &Path) -> bool { find_exe(llm_dir, "llama-server.exe").is_some() }

/// `subs_dir` 內是否已有任一可用模型 .bin（small/medium/turbo/large-v3）。
pub fn any_model_downloaded(subs_dir: &Path) -> bool {
    ["small", "medium", "turbo", "large-v3"]
        .iter()
        .any(|k| model_downloaded_in(subs_dir, k))
}

/// kind→大小（MB）。backend 依 backend_kind 區分 CUDA/非 CUDA（Vulkan 與 CPU 同走 CPU zip）。
pub fn asset_size_mb(kind: &str, backend_kind: &str) -> u64 {
    match kind {
        "model" => SIZE_MB_TURBO,
        "vad" => SIZE_MB_VAD,
        "ffmpeg" => SIZE_MB_FFMPEG,
        "backend" => {
            if backend_kind == "cuda" {
                SIZE_MB_BACKEND_CUDA
            } else {
                SIZE_MB_BACKEND_CPU
            }
        }
        _ => 0,
    }
}

/// 串流下載到 {name}.part、邊算 sha256、每 chunk 60s timeout、3 次重試、成功才 rename。哨符 <SKIP> 略過比對。
pub async fn download_verify(
    asset: &Asset,
    dest_dir: &Path,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, String> {
    tokio::fs::create_dir_all(dest_dir).await.map_err(|e| e.to_string())?;
    let final_path = dest_dir.join(asset.filename);
    let part = dest_dir.join(format!("{}.part", asset.filename));
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut last = String::new();
    for attempt in 0..3 {
        match try_download(&client, asset, &part, &mut on_progress).await {
            Ok(()) => {
                tokio::fs::rename(&part, &final_path).await.map_err(|e| e.to_string())?;
                return Ok(final_path);
            }
            Err(e) => {
                last = e;
                let _ = tokio::fs::remove_file(&part).await;
                tokio::time::sleep(Duration::from_secs(2 * (attempt + 1))).await;
            }
        }
    }
    Err(format!("download failed after retries: {last}"))
}

async fn try_download(
    client: &reqwest::Client,
    asset: &Asset,
    part: &Path,
    on_progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    use futures_util::StreamExt;
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncWriteExt;
    let resp = client.get(asset.url).send().await.map_err(|e| e.to_string())?;
    let total = resp.content_length();
    let mut hasher = Sha256::new();
    let mut file = tokio::fs::File::create(part).await.map_err(|e| e.to_string())?;
    let (mut done, mut last_emit) = (0u64, 0u64);
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::time::timeout(Duration::from_secs(60), stream.next())
            .await
            .map_err(|_| "stalled".to_string())?;
        let Some(chunk) = next else { break };
        let chunk = chunk.map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if done - last_emit >= 4_000_000 {
            on_progress(done, total);
            last_emit = done;
        }
    }
    file.flush().await.map_err(|e| e.to_string())?;
    on_progress(done, total);
    if asset.sha256 != "<SKIP>" {
        let got = format!("{:x}", hasher.finalize());
        if got != asset.sha256 {
            return Err(format!("sha256 mismatch {}: {got}", asset.filename));
        }
    }
    Ok(())
}

pub fn unzip(zip_path: &Path, dir: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    zip::ZipArchive::new(f)
        .map_err(|e| e.to_string())?
        .extract(dir)
        .map_err(|e| e.to_string())
}

/// 解壓後遞迴找指定檔名（應對版本子目錄、Release/ 等）。
pub fn find_exe(root: &Path, name: &str) -> Option<PathBuf> {
    walk(root)
        .into_iter()
        .find(|e| e.file_name().and_then(|s| s.to_str()) == Some(name))
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn emit_md(app: &AppHandle, key: &str, phase: &str, done: u64, total: Option<u64>, message: Option<String>) {
    let _ = app.emit(
        "model-download",
        super::ModelDownloadEvent { key: key.to_string(), phase: phase.to_string(), done, total, message },
    );
}

/// 認領擁有權的 RAII 守衛：Drop（含 tokio abort 丟棄 future 時）必移除 in-flight key。
pub struct InFlightGuard {
    set: Arc<Mutex<HashSet<String>>>,
    key: String,
}
impl Drop for InFlightGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = self.set.lock() {
            s.remove(&self.key);
        }
    }
}

/// 確保某模型 .bin 就緒（缺才下載），同一 key 去重、不同 key 並行。
/// emit_progress=true（手動 download_model）才 emit「進行中」model-download；terminal(done/error)一律 emit。
/// abort 安全：擁有權用 InFlightGuard，future 被 abort 丟棄時 Drop 仍釋放 key。
pub async fn ensure_model_file(
    app: &AppHandle,
    subs_dir: &Path,
    downloading: &Arc<Mutex<HashSet<String>>>,
    key: &str,
    emit_progress: bool,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, String> {
    let asset = model_asset(model_from_key(key));
    let path = subs_dir.join(asset.filename);
    loop {
        if path.exists() {
            emit_md(app, key, "done", 0, None, None);   // terminal、任何呼叫者都收到結束（無孤兒）
            return Ok(path);
        }
        let claimed = downloading.lock().map_err(|e| e.to_string())?.insert(key.to_string());
        if claimed {
            let _guard = InFlightGuard { set: downloading.clone(), key: key.to_string() };
            if emit_progress {
                emit_md(app, key, "downloading", 0, None, None);
            }
            let app2 = app.clone();
            let key2 = key.to_string();
            let r = download_verify(&asset, subs_dir, move |done, total| {
                if emit_progress {
                    emit_md(&app2, &key2, "downloading", done, total, None);
                }
                on_progress(done, total);
            })
            .await;
            return match r {
                Ok(p) => {
                    emit_md(app, key, "done", 0, None, None);
                    Ok(p)
                }
                Err(e) => {
                    emit_md(app, key, "error", 0, None, Some(e.clone()));
                    Err(e)
                }
            };
            // _guard 在此 drop（含提早 return / await 被 abort）→ remove(key)
        } else {
            // 別人持有同 key：等待者不 emit；睡一下回頂端，檔出現即回、或擁有者釋出後重新認領（自癒）。
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }
}

/// 確保 llama-server.exe + GGUF 就緒（缺才下載），with InFlightGuard 去重。
/// 不呼叫 ensure_model_file（whisper-寫死路徑），直接走 download_verify 新路徑。
pub async fn ensure_llm_assets(
    app: &AppHandle,
    llm_dir: &Path,
    downloading: &Arc<Mutex<HashSet<String>>>,
    backend: Backend,
    translate_key: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<(PathBuf, PathBuf), String> {
    tokio::fs::create_dir_all(llm_dir).await.map_err(|e| e.to_string())?;
    if find_exe(llm_dir, "llama-server.exe").is_none() {
        let key = "llm-server".to_string();
        let claimed = downloading.lock().map_err(|e| e.to_string())?.insert(key.clone());
        if claimed {
            let _g = InFlightGuard { set: downloading.clone(), key: key.clone() };
            let asset = llama_backend_asset(backend);
            let zip = download_verify(&asset, llm_dir, &mut on_progress).await?;
            unzip(&zip, llm_dir)?;
            let _ = tokio::fs::remove_file(&zip).await;
            emit_md(app, &key, "done", 0, None, None);
        } else {
            for _ in 0..600 {
                if find_exe(llm_dir, "llama-server.exe").is_some() { break; }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
    // CUDA 需要額外的 runtime DLL（binaries zip 不含）；解到同一 llm_dir 讓 llama-server.exe 能找到。
    if matches!(backend, Backend::Cuda) && !llm_dir.join("cudart64_12.dll").exists() {
        let cud = cudart_asset();
        let zip = download_verify(&cud, llm_dir, &mut on_progress).await?;
        unzip(&zip, llm_dir)?;
        let _ = tokio::fs::remove_file(&zip).await;
    }
    let exe = find_exe(llm_dir, "llama-server.exe").ok_or("llama-server.exe 缺")?;
    let asset = translate_model_asset(translate_key);
    let gguf = llm_dir.join(asset.filename);
    if !gguf.exists() {
        let key = translate_key.to_string();
        let claimed = downloading.lock().map_err(|e| e.to_string())?.insert(key.clone());
        if claimed {
            let _g = InFlightGuard { set: downloading.clone(), key: key.clone() };
            download_verify(asset, llm_dir, &mut on_progress).await?;
            emit_md(app, &key, "done", 0, None, None);
        } else {
            for _ in 0..1200 {
                if gguf.exists() { break; }
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        }
    }
    Ok((exe, gguf))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_downloaded_checks_correct_filename() {
        let dir = std::env::temp_dir().join(format!("lmpv-mtest-{}", std::process::id()));
        let subs = dir.join("subs");
        std::fs::create_dir_all(&subs).unwrap();
        assert!(!model_downloaded_in(&subs, "turbo"));
        std::fs::write(subs.join("ggml-large-v3-turbo.bin"), b"x").unwrap();
        assert!(model_downloaded_in(&subs, "turbo"));
        assert!(!model_downloaded_in(&subs, "small"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn any_model_downloaded_detects_any_key() {
        let dir = std::env::temp_dir().join(format!("lmpv-anymodel-{}", std::process::id()));
        let subs = dir.join("subs");
        std::fs::create_dir_all(&subs).unwrap();
        assert!(!any_model_downloaded(&subs));
        std::fs::write(subs.join("ggml-medium.bin"), b"x").unwrap();
        assert!(any_model_downloaded(&subs));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn asset_size_mb_maps_kinds() {
        assert_eq!(asset_size_mb("backend", "cuda"), SIZE_MB_BACKEND_CUDA);
        assert_eq!(asset_size_mb("backend", "vulkan"), SIZE_MB_BACKEND_CPU);
        assert_eq!(asset_size_mb("backend", "cpu"), SIZE_MB_BACKEND_CPU);
        assert_eq!(asset_size_mb("model", "cuda"), SIZE_MB_TURBO);
        assert_eq!(asset_size_mb("vad", "cpu"), SIZE_MB_VAD);
        assert_eq!(asset_size_mb("ffmpeg", "cpu"), SIZE_MB_FFMPEG);
        assert_eq!(asset_size_mb("bogus", "cpu"), 0);
    }

    #[test]
    fn cudart_asset_has_versioned_url() {
        let a = cudart_asset();
        assert!(a.url.contains("/b9843/"), "cudart URL should be pinned to release b9843");
        assert!(a.url.ends_with(".zip"));
        assert_eq!(a.filename, "cudart.zip");
        assert!(a.is_zip);
    }

    #[test]
    fn translate_model_from_key_maps() {
        assert_eq!(translate_model_from_key("translate-4b").filename, "translategemma-4b-it-Q4_K_M.gguf");
        assert_eq!(translate_model_from_key("translate-12b").filename, "translategemma-12b-it-Q4_K_M.gguf");
        // unknown → 4B default
        assert_eq!(translate_model_from_key("bogus").filename, "translategemma-4b-it-Q4_K_M.gguf");
    }

    #[test]
    fn ensure_llm_selects_gguf_by_key() {
        // The gguf chosen inside ensure_llm_assets is translate_model_asset(key).filename.
        assert_eq!(translate_model_asset("translate-12b").filename, "translategemma-12b-it-Q4_K_M.gguf");
        assert_eq!(translate_model_asset("translate-4b").filename, "translategemma-4b-it-Q4_K_M.gguf");
    }

    #[test]
    fn translate_model_downloaded_per_key() {
        let dir = std::env::temp_dir().join(format!("lmpv-tg-{}", std::process::id()));
        let llm = llm_dir(&dir);
        std::fs::create_dir_all(&llm).unwrap();
        assert!(!translate_model_downloaded(&llm, "translate-4b"));
        std::fs::write(llm.join("translategemma-4b-it-Q4_K_M.gguf"), b"x").unwrap();
        assert!(translate_model_downloaded(&llm, "translate-4b"));
        assert!(!translate_model_downloaded(&llm, "translate-12b"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
