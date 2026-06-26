<script setup lang="ts">
import { computed } from 'vue'
import { useSettings } from '../../player/useSettings'
import { useModelDownloads } from '../../player/useModelDownloads'
import { useEngineProvision } from '../../player/useEngineProvision'
import { rowState } from '../../player/modelRows'
import type { ModelKey } from '../../player/settings'
import { LANGS } from '../../player/langs'
import GlassSelect from '../GlassSelect.vue'
import GlassToggle from '../GlassToggle.vue'

const settings = useSettings()
const md = useModelDownloads()
const prov = useEngineProvision()

function onMaster(next: boolean) {
  if (next) prov.requestEnable()
  else prov.disableFeature()
}
const hasGpu = computed(() => settings.hw.info?.hasGpu ?? false)
const MODELS: { key: ModelKey; name: string; desc: string; vram: string }[] = [
  { key: 'turbo', name: '標準（large-v3-turbo）', desc: '推薦 · 多語言 · 快、品質佳', vram: '~2.5 GB VRAM' },
  { key: 'large-v3', name: '高品質（large-v3）', desc: '最準確、較慢', vram: '~4 GB VRAM' },
  { key: 'medium', name: '中等（medium）', desc: '準度與速度折衷', vram: '~2.5 GB VRAM' },
  { key: 'small', name: '輕量（small）', desc: '最快最省，品質普通', vram: '~1 GB VRAM' },
]
const LANG_OPTIONS = LANGS.map((l) => ({ value: l.value, label: l.label }))

const disabled = (key: ModelKey) => key === 'large-v3' && !hasGpu.value
const downloaded = (key: ModelKey) => md.downloaded.has(key)
const st = (key: ModelKey) =>
  rowState(key, settings.state.liveSubs.model, md.downloaded, new Set(md.downloading.keys()), md.errored)

const mb = (n: number) => Math.round(n / 1e6)
function progText(key: ModelKey): string {
  const p = md.downloading.get(key)
  if (!p) return '下載中…'
  if (p.total) return `${Math.round((p.done / p.total) * 100)}% · ${mb(p.done)} / ${mb(p.total)} MB`
  return p.done ? `${mb(p.done)} MB` : '下載中…'
}
function barWidth(key: ModelKey): string {
  const p = md.downloading.get(key)
  return p?.total ? `${Math.round((p.done / p.total) * 100)}%` : '0%'
}

function selectModel(key: ModelKey) {
  if (disabled(key) || !downloaded(key)) return
  settings.state.liveSubs.model = key
}

function resetVad() {
  settings.state.liveSubs.vad.threshold = 0.5
  settings.state.liveSubs.vad.minSilenceMs = 100
}
</script>

<template>
  <div>
    <div class="p-title">即時字幕</div>
    <div class="p-sub">AI 即時聽寫字幕（Whisper）。設定使用的模型、轉寫語言與自動儲存。</div>

    <div class="field">
      <div class="lab">啟用即時字幕<small>開啟後會自動下載尚未安裝的引擎（ffmpeg、語音偵測 VAD、轉譯模組(whisper)）</small></div>
      <GlassToggle :model-value="settings.state.liveSubs.enabled" @update:model-value="onMaster" />
    </div>

    <div v-if="settings.state.liveSubs.enabled">
    <div class="sec-h">模型</div>
    <div
      v-for="m in MODELS"
      :key="m.key"
      class="row"
      :class="{ active: st(m.key) === 'active', disabled: disabled(m.key) }"
      @click="selectModel(m.key)"
    >
      <div class="info">
        <div class="name">{{ m.name }}</div>
        <div class="desc">{{ m.desc }} · <span class="vram">{{ m.vram }}</span><span v-if="disabled(m.key)"> · 需 GPU</span></div>
      </div>
      <div class="act" @click.stop>
        <span v-if="st(m.key) === 'active' || st(m.key) === 'downloaded'" class="badge dn">✓ 已下載</span>
        <div v-else-if="st(m.key) === 'downloading'" class="prog">
          <div class="bar"><div :style="{ width: barWidth(m.key) }"></div></div>
          <div class="t">{{ progText(m.key) }}</div>
        </div>
        <template v-else>
          <button v-if="!disabled(m.key)" class="icon-btn" title="下載" @click="md.download(m.key)">
            <svg viewBox="0 0 24 24"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/></svg>
          </button>
          <span v-if="st(m.key) === 'error'" class="err">下載失敗，點擊重試</span>
        </template>
      </div>
    </div>

    <div class="sec-h">轉寫</div>
    <div class="field">
      <div class="lab">預設來源語言<small>聽寫時假設的語言；自動偵測通常就夠用</small></div>
      <GlassSelect :model-value="settings.state.liveSubs.sourceLang" :options="LANG_OPTIONS" @update:model-value="settings.state.liveSubs.sourceLang = $event" />
    </div>

    <div class="sec-h vad-h">語音偵測（VAD）<button class="reset-vad" @click="resetVad">恢復預設</button></div>
    <div class="field">
      <div class="lab">靈敏度<small>多大聲才算說話。漏掉小聲→調低；音樂/雜訊誤觸→調高。</small></div>
      <input type="range" min="0" max="1" step="0.05" v-model.number="settings.state.liveSubs.vad.threshold" />
      <span class="val">{{ settings.state.liveSubs.vad.threshold.toFixed(2) }}</span>
    </div>
    <div class="field">
      <div class="lab">斷句靜音門檻<small>靜音多久算一段結束。字幕太碎→調高；黏成一團→調低。</small></div>
      <input type="range" min="0" max="1000" step="50" v-model.number="settings.state.liveSubs.vad.minSilenceMs" />
      <span class="val">{{ settings.state.liveSubs.vad.minSilenceMs }} ms</span>
    </div>
    <div class="vad-note">改動會在下次開始轉寫時套用（換片或關開即時字幕）。</div>

    <div class="sec-h">即時字幕版面</div>
    <div class="field">
      <div class="lab">同時顯示行數<small>畫面上最多同時顯示幾行（含正在說的那行）。</small></div>
      <input type="range" min="2" max="5" step="1" v-model.number="settings.state.liveSubs.display.lines" />
      <span class="val">{{ settings.state.liveSubs.display.lines }} 行</span>
    </div>

    <div class="sec-h">儲存字幕</div>
    <div class="field">
      <div class="lab">自動儲存轉寫快取<small>轉好的字幕存進 app 內部快取，重播同片秒顯示、只補沒轉到的段落；不在影片旁留檔</small></div>
      <GlassToggle :model-value="settings.state.liveSubs.saveSrt" @update:model-value="settings.state.liveSubs.saveSrt = $event" />
    </div>
    <div class="field">
      <div class="lab">設定變更時重轉並覆寫快取<small>調整模型或 VAD 後重播同片，會用新設定重轉並覆寫舊快取；關閉則沿用舊快取</small></div>
      <GlassToggle
        :model-value="settings.state.liveSubs.overwriteOnParamChange"
        :disabled="!settings.state.liveSubs.saveSrt"
        @update:model-value="settings.state.liveSubs.overwriteOnParamChange = $event"
      />
    </div>

    </div>
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
.row.disabled { opacity: .45; cursor: not-allowed; }
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
input[type=range] {
  width: 150px; flex: none; height: 5px; -webkit-appearance: none; appearance: none;
  background: rgba(255,255,255,0.16); border-radius: 3px; cursor: pointer; outline: none;
}
input[type=range]::-webkit-slider-thumb {
  -webkit-appearance: none; appearance: none;
  width: 15px; height: 15px; border-radius: 50%; background: var(--accent);
  box-shadow: 0 1px 3px rgba(0,0,0,0.5); cursor: pointer;
}
.val { width: 56px; text-align: right; font-size: 12px; color: #cdd3da; flex: none; }
.vad-h { display: flex; align-items: center; justify-content: space-between; }
.reset-vad { background: none; border: none; color: var(--accent); font-size: 12px; cursor: pointer; padding: 0; }
.vad-note { font-size: 11px; color: #7a7a82; margin: 2px 2px 6px; }
</style>
