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
pub struct ProcessSource {
    pub pid: u32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>, // exe 圖示 PNG data URL（抽不到 → None，前端 fallback）
}

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
            let path = process_path(pid);
            let name = path
                .as_deref()
                .map(|p| p.rsplit(['\\', '/']).next().unwrap_or(p).to_string())
                .unwrap_or_else(|| format!("PID {pid}"));
            let icon = path.as_deref().and_then(extract_icon);
            out.push(ProcessSource { pid, name, icon });
        }
        Ok(out)
    }
}

/// PID → 執行檔完整路徑（供 basename + 圖示抽取）。
unsafe fn process_path(pid: u32) -> Option<String> {
    let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = [0u16; MAX_PATH as usize];
    let mut len = buf.len() as u32;
    let r = QueryFullProcessImageNameW(h, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut len);
    let _ = CloseHandle(h);
    r.ok()?;
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

/// 從 exe 抽出圖示 → PNG data URL。失敗回 None（前端 fallback 通用 icon）。
/// 流程：SHGetFileInfo 取 HICON → GetIconInfo 取彩色點陣 → GetDIBits 取 32bpp BGRA →
/// 轉 RGBA（無 alpha 的舊圖示補成不透明）→ png 編碼 → base64。
fn extract_icon(exe_path: &str) -> Option<String> {
    use base64::Engine;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HDC, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    let wide: Vec<u16> = exe_path.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let mut shfi = SHFILEINFOW::default();
        let r = SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut shfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if r == 0 || shfi.hIcon.is_invalid() {
            return None;
        }
        let hicon = shfi.hIcon;
        let mut ii = ICONINFO::default();
        if GetIconInfo(hicon, &mut ii).is_err() {
            let _ = DestroyIcon(hicon);
            return None;
        }
        let mut bm = BITMAP::default();
        let n = GetObjectW(
            HGDIOBJ(ii.hbmColor.0),
            std::mem::size_of::<BITMAP>() as i32,
            Some(&mut bm as *mut _ as *mut core::ffi::c_void),
        );
        let (w, h) = (bm.bmWidth, bm.bmHeight);
        let cleanup = || {
            let _ = DeleteObject(HGDIOBJ(ii.hbmColor.0));
            let _ = DeleteObject(HGDIOBJ(ii.hbmMask.0));
            let _ = DestroyIcon(hicon);
        };
        if n == 0 || w <= 0 || h <= 0 || w > 256 || h > 256 {
            cleanup();
            return None;
        }
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w;
        bi.bmiHeader.biHeight = -h; // top-down
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let hdc: HDC = GetDC(None);
        let got = GetDIBits(
            hdc,
            ii.hbmColor,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut bi,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        cleanup();
        if got == 0 {
            return None;
        }
        // BGRA → RGBA；舊圖示整片 alpha=0 → 補成不透明，否則整顆透明看不到。
        let any_alpha = buf.chunks_exact(4).any(|p| p[3] != 0);
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2);
            if !any_alpha {
                px[3] = 255;
            }
        }
        let mut png_buf: Vec<u8> = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_buf, w as u32, h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().ok()?;
            writer.write_image_data(&buf).ok()?;
        }
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&png_buf)
        ))
    }
}
