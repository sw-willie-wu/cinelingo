// Pinned, no-curl libmpv runtime — the single source of truth for which DLLs
// belong in src-tauri/lib/.
//
// WHY THIS EXISTS: `npx tauri-plugin-libmpv-api setup-lib` downloads zhongfly's
// `mpv-dev-lgpl` build from `latest`, which is linked against libcurl. The curl
// backend does NOT pre-read network sources, so 4K YouTube stalls ("play 2s,
// wait 10s") no matter how large the cache is. shinchiro's build uses the ffmpeg
// backend (no curl) and pre-reads correctly. We therefore provision the DLLs
// ourselves, pinned to an exact version + SHA256, so the verified no-curl binary
// is reproducible from scratch and can never be clobbered by `setup-lib`.
//
// To bump a version: update url/archiveSha256, download + extract the member,
// recompute its sha256, and confirm it is still a no-curl build before pinning.

// Path to src-tauri/lib, relative to the scripts/ directory.
export const LIB_DIR = ['..', 'src-tauri', 'lib']

export const LIBMPV_COMPONENTS = [
  {
    // mpv 0.41.0-744-g304426c39 — shinchiro mpv-winbuild-cmake, no-curl.
    name: 'libmpv-2.dll',
    url: 'https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20260610/mpv-dev-x86_64-20260610-git-304426c.7z',
    archiveSha256: '8cbb25ea784f01afbb3f904217cab1317430a8bcfd5680fd827a866367f71cc9',
    member: 'libmpv-2.dll',
    dllSha256: '5c876d79e070529128331591b48f87846fb30557f19c11280df9c6ee9b6dbafa',
  },
  {
    // nini22P/libmpv-wrapper v0.1.1 (matches tauri-plugin-libmpv 0.3.x ABI).
    name: 'libmpv-wrapper.dll',
    url: 'https://github.com/nini22P/libmpv-wrapper/releases/download/v0.1.1/libmpv-wrapper-windows-x86_64.zip',
    archiveSha256: 'd2ff8b2edcd34d2968e544adaa915e5e5c48eb1a0995945005269c2af119a492',
    member: 'bin/libmpv-wrapper.dll',
    dllSha256: '0d5adead5f175c55e0790a80924ec0a2636f72e3675c79a6d9d9568b2ed2384a',
  },
]
