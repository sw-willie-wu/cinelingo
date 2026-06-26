// Dev helper: copy libmpv DLLs next to the dev binary so the plugin can find them.
//
// `tauri dev` runs target/debug/app.exe, and tauri-plugin-libmpv only searches
// `current_exe()/` and `current_exe()/lib` for libmpv-2.dll / libmpv-wrapper.dll.
// The DLLs live in src-tauri/lib/ (fetched by `tauri-plugin-libmpv-api setup-lib`,
// gitignored). In a packaged build, bundle.resources handles this; in dev it does not.
// npm runs this automatically as the `predev` hook before `vite`.

import { createHash } from 'node:crypto'
import { mkdirSync, copyFileSync, existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { LIBMPV_COMPONENTS } from './libmpv-manifest.mjs'

const here = dirname(fileURLToPath(import.meta.url))
const srcLib = join(here, '..', 'src-tauri', 'lib')
const destDir = join(here, '..', 'src-tauri', 'target', 'debug')

if (!existsSync(srcLib)) {
  console.warn('[copy-libmpv] src-tauri/lib not found — run: npm run setup-libmpv')
  process.exit(0)
}

// Guard: never mirror a wrong/curl-linked libmpv into the dev binary. If a DLL
// doesn't match the pinned no-curl SHA256 (e.g. `setup-lib` clobbered it), fail
// loudly instead of silently shipping a build that stalls on 4K streams.
for (const c of LIBMPV_COMPONENTS) {
  const p = join(srcLib, c.name)
  if (!existsSync(p)) continue // setup-libmpv (run first by predev/prebuild) provisions it
  const got = createHash('sha256').update(readFileSync(p)).digest('hex')
  if (got !== c.dllSha256) {
    console.error(`[copy-libmpv] ${c.name} is not the pinned no-curl build.`)
    console.error(`  expected sha256 ${c.dllSha256}`)
    console.error(`  got      sha256 ${got}`)
    console.error('  Run `npm run setup-libmpv` to restore the verified DLL.')
    process.exit(1)
  }
}

mkdirSync(destDir, { recursive: true })

let copied = 0
for (const name of readdirSync(srcLib)) {
  if (!name.toLowerCase().endsWith('.dll')) continue
  const src = join(srcLib, name)
  const dest = join(destDir, name)
  if (existsSync(dest) && statSync(dest).size === statSync(src).size) continue // unchanged → skip
  copyFileSync(src, dest)
  console.log('[copy-libmpv] copied', name)
  copied++
}
if (copied === 0) console.log('[copy-libmpv] DLLs already up to date')
