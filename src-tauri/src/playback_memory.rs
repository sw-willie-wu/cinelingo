use std::path::PathBuf;

fn memory_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(crate::data_dir(app)?.join("playback-memory.json"))
}

/// per-video 播放記憶（speed/audioDelay）。缺檔/壞檔 → 空物件。
#[tauri::command]
pub async fn load_playback_memory(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let p = memory_path(&app)?;
    match tokio::fs::read(&p).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}))),
        Err(_) => Ok(serde_json::json!({})),
    }
}

/// 寫整個記憶 map（tmp+rename 原子寫；建目錄）。
#[tauri::command]
pub async fn save_playback_memory(app: tauri::AppHandle, data: serde_json::Value) -> Result<(), String> {
    let p = memory_path(&app)?;
    if let Some(dir) = p.parent() {
        tokio::fs::create_dir_all(dir).await.map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?;
    tokio::fs::write(&tmp, bytes).await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, &p).await.map_err(|e| e.to_string())?;
    Ok(())
}
