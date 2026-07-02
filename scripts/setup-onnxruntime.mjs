// Provision the pinned onnxruntime.dll into src-tauri/lib/.
//
// The ort crate's build-time download is suppressed via ORT_SKIP_DOWNLOAD=1 in
// src-tauri/.cargo/config.toml; this script provides the exact DLL instead,
// verified against a pinned archive + DLL SHA256 pair (see onnxruntime-manifest.mjs).
//
// Idempotent: if lib/onnxruntime.dll already matches the pinned SHA256 it is skipped.
// Wired into the `predev` / `prebuild` npm hooks, so `npm run dev` and
// `npm run tauri build` always start from the verified binary.

import { createHash } from 'node:crypto'
import {
  mkdirSync, existsSync, readFileSync, writeFileSync, copyFileSync, rmSync, mkdtempSync,
} from 'node:fs'
import { join, dirname, basename } from 'node:path'
import { fileURLToPath } from 'node:url'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { ONNXRUNTIME } from './onnxruntime-manifest.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const libDir = join(here, '..', 'src-tauri', 'lib')

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

// 7-Zip is required to unpack the .zip archive. Try PATH first, then the
// standard install locations.
function find7z() {
  const candidates = [
    '7z', '7za',
    'C:\\Program Files\\7-Zip\\7z.exe',
    'C:\\Program Files (x86)\\7-Zip\\7z.exe',
  ]
  for (const c of candidates) {
    const r = spawnSync(c, [], { stdio: 'ignore' })
    if (!r.error) return c // exists (prints usage); only ENOENT sets r.error
  }
  return null
}

async function download(url, dest) {
  const res = await fetch(url, { redirect: 'follow' })
  if (!res.ok) throw new Error(`HTTP ${res.status} ${res.statusText} for ${url}`)
  writeFileSync(dest, Buffer.from(await res.arrayBuffer()))
}

// `7z e` flattens archive paths, so the extracted file lands at outDir/<basename>.
function extractMember(sevenZip, archive, member, outDir) {
  const r = spawnSync(sevenZip, ['e', '-y', `-o${outDir}`, archive, member], { stdio: 'ignore' })
  if (r.status !== 0) throw new Error(`7z failed to extract ${member} from ${basename(archive)} (status ${r.status})`)
  const out = join(outDir, basename(member))
  if (!existsSync(out)) throw new Error(`7z reported success but ${basename(member)} was not produced`)
  return out
}

async function main() {
  mkdirSync(libDir, { recursive: true })

  const dest = join(libDir, ONNXRUNTIME.name)
  if (existsSync(dest) && sha256(dest) === ONNXRUNTIME.dllSha256) {
    console.log('[setup-onnxruntime] onnxruntime.dll present and verified — skipping download')
    return
  }

  const sevenZip = find7z()
  if (!sevenZip) {
    console.error('[setup-onnxruntime] 7-Zip not found. Install it (https://www.7-zip.org/ or `winget install 7zip.7zip`), then re-run `npm run setup-onnxruntime`.')
    process.exit(1)
  }

  console.log(`[setup-onnxruntime] provisioning ${ONNXRUNTIME.name} …`)
  const work = mkdtempSync(join(tmpdir(), 'ort-setup-'))
  try {
    const archivePath = join(work, basename(ONNXRUNTIME.url))
    await download(ONNXRUNTIME.url, archivePath)

    const archiveHash = sha256(archivePath)
    if (archiveHash !== ONNXRUNTIME.archiveSha256) {
      throw new Error(
        `archive SHA256 mismatch for ${ONNXRUNTIME.name}\n  expected ${ONNXRUNTIME.archiveSha256}\n  got      ${archiveHash}`,
      )
    }

    const outDir = join(work, 'out')
    mkdirSync(outDir, { recursive: true })
    const extracted = extractMember(sevenZip, archivePath, ONNXRUNTIME.member, outDir)

    const dllHash = sha256(extracted)
    if (dllHash !== ONNXRUNTIME.dllSha256) {
      throw new Error(
        `DLL SHA256 mismatch for ${ONNXRUNTIME.name}\n  expected ${ONNXRUNTIME.dllSha256}\n  got      ${dllHash}`,
      )
    }

    copyFileSync(extracted, dest)
    console.log(`[setup-onnxruntime] ${ONNXRUNTIME.name} OK (sha256 ${ONNXRUNTIME.dllSha256.slice(0, 12)}…)`)
    console.log('[setup-onnxruntime] done — pinned onnxruntime.dll installed in src-tauri/lib/')
  } finally {
    rmSync(work, { recursive: true, force: true })
  }
}

main().catch((e) => {
  console.error('[setup-onnxruntime] FAILED:', e.message)
  process.exit(1)
})
