// Stamp the app version across package.json, src-tauri/tauri.conf.json, and
// src-tauri/Cargo.toml. CI runs this with the git tag's version so the built
// installer/exe metadata matches the tag (single source of truth = the tag).
//
// Usage: node scripts/set-version.mjs <major.minor.patch[-prerelease]>

import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const version = process.argv[2]
const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/
if (!version || !SEMVER.test(version)) {
  console.error(`[set-version] invalid or missing semver: "${version ?? ''}"`)
  console.error('  usage: node scripts/set-version.mjs <major.minor.patch[-prerelease]>')
  process.exit(1)
}

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

function setJsonVersion(relPath) {
  const p = join(root, relPath)
  const json = JSON.parse(readFileSync(p, 'utf8'))
  json.version = version
  writeFileSync(p, `${JSON.stringify(json, null, 2)}\n`)
  console.log(`[set-version] ${relPath} -> ${version}`)
}

function setCargoVersion(relPath) {
  const p = join(root, relPath)
  const txt = readFileSync(p, 'utf8')
  // Only the [package] version is a line starting with `version = "..."`;
  // dependency versions are inline-table values (`x = { version = ... }`).
  if (!/^version = "[^"]*"/m.test(txt)) {
    console.error(`[set-version] no [package] version line in ${relPath}`)
    process.exit(1)
  }
  writeFileSync(p, txt.replace(/^version = "[^"]*"/m, `version = "${version}"`))
  console.log(`[set-version] ${relPath} -> ${version}`)
}

setJsonVersion('package.json')
setJsonVersion('src-tauri/tauri.conf.json')
setCargoVersion('src-tauri/Cargo.toml')
console.log(`[set-version] done: ${version}`)
