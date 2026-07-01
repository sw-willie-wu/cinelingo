use std::ffi::OsString;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[allow(dead_code)]
pub const DEFAULT_CTX: u32 = 4096;

#[allow(dead_code)]
pub fn build_server_args(gguf: &Path, port: u16, ctx: u32) -> Vec<OsString> {
    vec![
        OsString::from("-m"), gguf.into(),
        OsString::from("-ngl"), OsString::from("99"),
        OsString::from("-c"), OsString::from(ctx.to_string()),
        OsString::from("-fa"), OsString::from("on"),
        OsString::from("--reasoning-budget"), OsString::from("0"),
        OsString::from("--host"), OsString::from("127.0.0.1"),
        OsString::from("--port"), OsString::from(port.to_string()),
    ]
}

#[allow(dead_code)]
pub struct LlamaServer {
    child: Mutex<Option<Child>>,
    pub port: u16,
}

#[allow(dead_code)]
impl LlamaServer {
    pub async fn start(exe: &Path, gguf: &Path) -> Result<Self, String> {
        let port = super::super::whisper::free_port()?; // whisper::free_port 為 pub
        let mut cmd = Command::new(exe);
        cmd.args(build_server_args(gguf, port, DEFAULT_CTX))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW：不彈 console 視窗（tokio Command 內建，不需 import）
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let http = reqwest::Client::new();
        for _ in 0..240 {
            // 4B 模型載入可能久 ~120s
            if let Ok(Some(st)) = child.try_wait() {
                return Err(format!("llama-server exited early: {st}"));
            }
            if http
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                return Ok(Self {
                    child: Mutex::new(Some(child)),
                    port,
                });
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let _ = child.kill().await;
        Err("llama-server health timeout".into())
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// &self → 可從 Arc 呼叫；idempotent（取出 child 一次）。
    pub async fn kill(&self) {
        if let Some(mut c) = self.child.lock().await.take() {
            let _ = c.kill().await;
            let _ = c.wait().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_have_model_port_and_reasoning_off() {
        let a = build_server_args(Path::new("m.gguf"), 8899, 4096);
        let s: Vec<String> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
        assert!(s.windows(2).any(|w| w[0] == "-m" && w[1] == "m.gguf"));
        assert!(s.windows(2).any(|w| w[0] == "--port" && w[1] == "8899"));
        assert!(s.windows(2).any(|w| w[0] == "--reasoning-budget" && w[1] == "0"));
    }
}
