use std::path::{Path, PathBuf};
use tokio::process::Command;

/// 解碼指定 ff-index 音軌為 16kHz mono s16 wav。
pub async fn decode_track(ffmpeg: &Path, input: &str, ff_index: i64, dest: &Path) -> Result<PathBuf, String> {
    let out = dest.to_path_buf();
    let mut cmd = Command::new(ffmpeg);
    cmd.args([
        "-y", "-i", input, "-map", &format!("0:{ff_index}"),
        "-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav",
    ])
    .arg(&out)
    .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：GUI app 起子行程不彈 console 視窗
    let st = cmd.status().await.map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(format!("ffmpeg decode exit {st}"));
    }
    Ok(out)
}

/// 從整軌 wav 取 [start, start+dur] 小 wav。輸入端 -ss 對 PCM WAV 為 sample-accurate；用 -t 指定長度避免 -to 歧義。
pub async fn slice_window(ffmpeg: &Path, track_wav: &Path, start: f64, end: f64, dest: &Path) -> Result<PathBuf, String> {
    let out = dest.to_path_buf();
    let dur = (end - start).max(0.0);
    let mut cmd = Command::new(ffmpeg);
    cmd.args(["-y", "-ss", &start.to_string(), "-t", &dur.to_string(), "-i"])
        .arg(track_wav)
        .args(["-c", "copy", "-f", "wav"])
        .arg(&out)
        .kill_on_drop(true);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：每塊切音不彈 console 視窗
    let st = cmd.status().await.map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(format!("ffmpeg slice exit {st}"));
    }
    Ok(out)
}
