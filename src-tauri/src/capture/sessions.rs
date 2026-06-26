#![allow(unsafe_code)]
use serde::Serialize;
use windows::core::Interface;
use windows::Win32::Media::Audio::{
    IMMDeviceEnumerator, MMDeviceEnumerator, eRender, eConsole,
    IAudioSessionManager2, IAudioSessionControl2,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_NAME_FORMAT};
use windows::Win32::Foundation::{CloseHandle, MAX_PATH};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessSource { pub pid: u32, pub name: String }

/// 列出預設 render endpoint 上「有 audio session 的程式」。wasapi 0.19 不提供此列舉 → 走原生 COM。
/// 呼叫端須已 CoInitialize（wasapi::initialize_mta() 已做；或 spawn_blocking 內先 init）。
pub fn list_audio_processes() -> Result<Vec<ProcessSource>, String> {
    let _ = wasapi::initialize_mta(); // 與既有 capture COM 對齊
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|e| e.to_string())?;
        let mgr: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
        let sessions = mgr.GetSessionEnumerator().map_err(|e| e.to_string())?;
        let count = sessions.GetCount().map_err(|e| e.to_string())?;
        let mut out: Vec<ProcessSource> = Vec::new();
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for i in 0..count {
            let ctrl = match sessions.GetSession(i) { Ok(c) => c, Err(_) => continue };
            let ctrl2: IAudioSessionControl2 = match ctrl.cast() { Ok(c) => c, Err(_) => continue };
            let pid = ctrl2.GetProcessId().unwrap_or(0);
            if pid == 0 || pid == std::process::id() || !seen.insert(pid) { continue; } // 跳系統音效/自己/去重
            let name = process_name(pid).unwrap_or_else(|| format!("PID {pid}"));
            out.push(ProcessSource { pid, name });
        }
        Ok(out)
    }
}

/// PID → 執行檔 basename（GetDisplayName 常為空 → 用此）。
unsafe fn process_name(pid: u32) -> Option<String> {
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; MAX_PATH as usize];
    let mut len = buf.len() as u32;
    let r = QueryFullProcessImageNameW(h, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
    let _ = CloseHandle(h);
    r.ok()?;
    let full = String::from_utf16_lossy(&buf[..len as usize]);
    Some(full.rsplit(['\\', '/']).next().unwrap_or(&full).to_string())
}
