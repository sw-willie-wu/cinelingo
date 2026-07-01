// Pinned onnxruntime.dll (CPU, Windows x64) — the single source of truth.
//
// WHY THIS EXISTS: `ort` crate's `download-binaries` feature (pulled in via
// `voice_activity_detector`) causes `ort-sys` build.rs to download onnxruntime
// at build time into %LOCALAPPDATA%/ort.pyke.io/dfbin/. That download is
// suppressed by setting ORT_SKIP_DOWNLOAD=1 (see src-tauri/.cargo/config.toml),
// and instead we provision the exact DLL ourselves — pinned to a specific version
// and verified by double SHA256 — so the binary is reproducible from scratch and
// no network access is needed during cargo build.
//
// Version alignment: onnxruntime 1.22.0 matches the `ms@1.22.0` that
// voice_activity_detector 0.2.1 pins via `ort =2.0.0-rc.10`.
//
// To bump a version: update url/archiveSha256, download + extract the member,
// recompute its sha256, update dllSha256, and re-verify the runtime still works.

export const ONNXRUNTIME = {
  name: 'onnxruntime.dll',
  url: 'https://github.com/microsoft/onnxruntime/releases/download/v1.22.0/onnxruntime-win-x64-1.22.0.zip',
  archiveSha256: '174c616efc0271194488642a72f1a514e01487da4dfe84c49296d66e40ebe0da',
  member: 'onnxruntime-win-x64-1.22.0/lib/onnxruntime.dll',
  dllSha256: '579b636403983254346a5c1d80bd28f1519cd1e284cd204f8d4ff41f8d711559',
}
