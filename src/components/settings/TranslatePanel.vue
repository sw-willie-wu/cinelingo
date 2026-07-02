<script setup lang="ts">
import { computed } from 'vue'
import { useSettings } from '../../player/useSettings'
import { useModelDownloads } from '../../player/useModelDownloads'
import { useTranslateProvision } from '../../player/useTranslateProvision'
import { rowState } from '../../player/modelRows'
import { LANGS } from '../../player/langs'
import { provisionTranslateEngine } from '../../player/backend'
import GlassSelect from '../GlassSelect.vue'
import GlassToggle from '../GlassToggle.vue'

const settings = useSettings()
const md = useModelDownloads()
const prov = useTranslateProvision()

type TransKey = 'translate-4b' | 'translate-12b'
const TRANS_MODELS: { key: TransKey; name: string; desc: string; vram: string }[] = [
  { key: 'translate-4b', name: '標準（TranslateGemma 4B）', desc: '推薦 · 與字幕同跑 live 安全', vram: '~5.5 GB（含字幕）' },
  { key: 'translate-12b', name: '高品質（TranslateGemma 12B）', desc: '翻譯更準；需大顯存', vram: '~8 GB · 與字幕同跑 ≤10GB 卡可能 OOM' },
]
const mb = (n: number) => Math.round(n / 1e6)
const selectedTrans = computed<TransKey>(() => settings.state.liveSubs.translateModel as TransKey)
const TRANS_LANG_OPTIONS = LANGS.filter((l) => l.value !== 'auto').map((l) => ({ value: l.value, label: l.label }))
const transSt = (key: TransKey) =>
  rowState(key, selectedTrans.value, md.downloaded, new Set(md.downloading.keys()), md.errored)
function selectTrans(key: TransKey) {
  if (!md.downloaded.has(key)) return
  settings.state.liveSubs.translateModel = key
}
function transProgText(key: TransKey): string {
  const p = md.downloading.get(key)
  if (!p) return '下載中…'
  if (p.total) return `${Math.round((p.done / p.total) * 100)}% · ${mb(p.done)} / ${mb(p.total)} MB`
  return p.done ? `${mb(p.done)} MB` : '下載中…'
}
function transBarWidth(key: TransKey): string {
  const p = md.downloading.get(key)
  return p?.total ? `${Math.round((p.done / p.total) * 100)}%` : '0%'
}
async function downloadTrans(key: TransKey) {
  if (md.downloaded.has(key) || md.downloading.get(key)) return
  md.downloading.set(key, { done: 0, total: null })
  try { await provisionTranslateEngine(key) } catch { md.downloading.delete(key) }
}
function onMaster(on: boolean) {
  if (on) void prov.requestEnable()
  else prov.disable()
}
</script>

<template>
  <div>
    <div class="p-title">字幕翻譯</div>
    <div class="p-sub">用本地 TranslateGemma 把即時聽寫字幕翻成目標語言（翻譯與聽寫共用 GPU）。</div>

    <div class="field">
      <div class="lab">啟用翻譯<small>開啟後會備妥下方選定的模型；各軌可在「⋯ → 字幕」逐軌選「翻譯成」</small></div>
      <GlassToggle :model-value="settings.state.liveSubs.translateEnabled" @update:model-value="onMaster($event)" />
    </div>

    <template v-if="settings.state.liveSubs.translateEnabled">
      <div class="sec-h">模型</div>
      <div
        v-for="m in TRANS_MODELS"
        :key="m.key"
        class="row"
        :class="{ active: transSt(m.key) === 'active' }"
        @click="selectTrans(m.key)"
      >
        <div class="info">
          <div class="name">{{ m.name }}</div>
          <div class="desc">{{ m.desc }} · <span class="vram">{{ m.vram }}</span></div>
        </div>
        <div class="act" @click.stop>
          <span v-if="transSt(m.key) === 'active' || transSt(m.key) === 'downloaded'" class="badge dn">✓ 已下載</span>
          <div v-else-if="transSt(m.key) === 'downloading'" class="prog">
            <div class="bar"><div :style="{ width: transBarWidth(m.key) }"></div></div>
            <div class="t">{{ transProgText(m.key) }}</div>
          </div>
          <template v-else>
            <button class="icon-btn" title="下載翻譯模型" @click="downloadTrans(m.key)">
              <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>
            </button>
            <span v-if="transSt(m.key) === 'error'" class="err">下載失敗，點擊重試</span>
          </template>
        </div>
      </div>

      <div class="field">
        <div class="lab">預設翻譯語言<small>逐軌開啟「翻譯成」時的預設目標</small></div>
        <GlassSelect :model-value="settings.state.liveSubs.translateTo" :options="TRANS_LANG_OPTIONS" @update:model-value="settings.state.liveSubs.translateTo = $event" />
      </div>
    </template>
  </div>
</template>

<style scoped>
.p-title { font-size: 15px; font-weight: 600; color: #fff; margin: 0 0 3px; }
.p-sub { font-size: 12px; color: #8a8a92; margin: 0 0 14px; }
.sec-h { font-size: 11px; letter-spacing: .4px; text-transform: uppercase; color: #7fb2ff; font-weight: 600; margin: 18px 0 9px; border-bottom: 1px solid rgba(255,255,255,0.08); padding-bottom: 5px; }
.sec-h:first-of-type { margin-top: 4px; }
.row { display: flex; align-items: center; gap: 12px; padding: 11px 13px; border: 1px solid rgba(255,255,255,0.09); border-radius: 9px; margin-bottom: 7px; background: rgba(255,255,255,0.045); cursor: pointer; transition: background .12s, border-color .12s; }
.row:hover { border-color: rgba(255,255,255,0.18); background: rgba(255,255,255,0.07); }
.row.active { border-color: var(--accent); background: rgba(var(--accent-rgb),0.16); box-shadow: 0 0 0 1px var(--accent) inset; }
.info { flex: 1; min-width: 0; }
.name { font-weight: 600; color: #fff; }
.desc { font-size: 11px; color: #8f8f97; margin-top: 2px; }
.vram { color: #6f7178; }
.badge { font-size: 11px; padding: 4px 12px; border-radius: 999px; }
.badge.dn { background: #143524; color: #4ade80; border: 1px solid #1f5036; }
.act { flex: none; display: flex; align-items: center; justify-content: flex-end; gap: 8px; min-width: 150px; }
.icon-btn { width: 32px; height: 32px; border: none; background: none; color: #9aa0a8; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: color .15s; padding: 0; }
.icon-btn:hover { color: var(--accent); }
.icon-btn svg { width: 18px; height: 18px; stroke: currentColor; fill: none; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.prog { width: 150px; }
.prog .bar { height: 6px; background: rgba(255,255,255,0.14); border-radius: 3px; overflow: hidden; }
.prog .bar > div { height: 100%; background: var(--accent); border-radius: 3px; transition: width .2s ease; }
.prog .t { font-size: 10px; color: #9fc6ff; margin-top: 4px; text-align: right; white-space: nowrap; }
.err { font-size: 11px; color: #ff8a8a; white-space: nowrap; }
.field { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 9px 2px; }
.lab { color: #d8d8de; flex: 1; }
.lab small { display: block; color: #7a7a82; font-size: 11px; margin-top: 1px; }
</style>
