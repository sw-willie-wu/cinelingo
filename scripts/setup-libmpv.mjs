// Provision the PINNED, no-curl libmpv DLLs into src-tauri/lib/.
//
// Replaces `npx tauri-plugin-libmpv-api setup-lib` (which pulls a libcurl-linked
// build from `latest` and stalls on 4K streams — see libmpv-manifest.mjs). Each
// DLL is downloaded by exact URL, verified against a pinned archive + DLL
// SHA256, then placed in lib/.
//
// Idempotent + self-healing: a component whose lib/ copy already matches the
// pinned SHA256 is skipped; one that is missing or mismatched (e.g. clobbered by
// `setup-lib`) is re-fetched. Wired into the `predev` / `prebuild` npm hooks, so
// `npm run dev` and `npm run tauri build` always start from the verified binary.

import { createHash } from 'node:crypto'
import {
  mkdirSync, existsSync, readFileSync, writeFileSync, copyFileSync, rmSync, mkdtempSync,
} from 'node:fs'
import { join, dirname, basename } from 'node:path'
import { fileURLToPath } from 'node:url'
import { tmpdir } from 'node:os'
import { spawnSync } from 'node:child_process'
import { LIBMPV_COMPONENTS, LIB_DIR } from './libmpv-manifest.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const libDir = join(here, ...LIB_DIR)

function sha256(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex')
}

// 7-Zip is required to unpack mpv's .7z archive. Try PATH first, then the
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

  const needWork = LIBMPV_COMPONENTS.filter((c) => {
    const dest = join(libDir, c.name)
    return !(existsSync(dest) && sha256(dest) === c.dllSha256)
  })
  if (needWork.length === 0) {
    console.log('[setup-libmpv] all DLLs present and verified (pinned no-curl)')
    return
  }

  const sevenZip = find7z()
  if (!sevenZip) {
    console.error('[setup-libmpv] 7-Zip not found. Install it (https://www.7-zip.org/ or `winget install 7zip.7zip`), then re-run `npm run setup-libmpv`.')
    process.exit(1)
  }

  const work = mkdtempSync(join(tmpdir(), 'libmpv-setup-'))
  try {
    for (const c of needWork) {
      console.log(`[setup-libmpv] provisioning ${c.name} …`)
      const archivePath = join(work, basename(c.url))
      await download(c.url, archivePath)

      const archiveHash = sha256(archivePath)
      if (archiveHash !== c.archiveSha256) {
        throw new Error(`archive SHA256 mismatch for ${c.name}\n  expected ${c.archiveSha256}\n  got      ${archiveHash}`)
      }

      const outDir = join(work, `${c.name}.out`)
      mkdirSync(outDir, { recursive: true })
      const extracted = extractMember(sevenZip, archivePath, c.member, outDir)

      const dllHash = sha256(extracted)
      if (dllHash !== c.dllSha256) {
        throw new Error(`DLL SHA256 mismatch for ${c.name}\n  expected ${c.dllSha256}\n  got      ${dllHash}`)
      }

      copyFileSync(extracted, join(libDir, c.name))
      console.log(`[setup-libmpv] ${c.name} OK (sha256 ${c.dllSha256.slice(0, 12)}…)`)
    }
    console.log('[setup-libmpv] done — pinned no-curl libmpv installed in src-tauri/lib/')
  } finally {
    rmSync(work, { recursive: true, force: true })
  }
}

main().catch((e) => {
  console.error('[setup-libmpv] FAILED:', e.message)
  process.exit(1)
})
