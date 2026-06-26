<script setup lang="ts">
import { computed } from 'vue'
import { useEngineProvision } from '../player/useEngineProvision'
import { useSubtitles } from '../player/useSubtitles'

const prov = useEngineProvision()
const subs = useSubtitles()

const KIND_LABEL: Record<string, string> = {
  model: '語音模型（turbo）',
  vad: '語音偵測（VAD）',
  backend: '轉譯模組(whisper)',
  ffmpeg: 'ffmpeg',
}
const gb = (mb: number) => (mb / 1024).toFixed(1)
const mb = (n: number) => Math.round(n / 1e6)

// 目前下載項目 + %（單一來源：sub-progress）。
const current = computed(() => {
  const p = subs.progress.value
  if (!p || !['model', 'vad', 'backend', 'ffmpeg'].includes(p.phase)) return null
  const label = KIND_LABEL[p.phase] ?? p.phase
  if (p.total) {
    const pct = Math.round((p.done / p.total) * 100)
    return { label, pct, text: `${pct}% · ${mb(p.done)} / ${mb(p.total)} MB` }
  }
  return { label, pct: 0, text: '準備中…' }
})
</script>

<template>
  <div v-if="prov.state.open" class="ep-backdrop">
    <div class="ep">
      <!-- 詢問 -->
      <template v-if="prov.state.phase === 'ask'">
        <div class="ep-title">啟用即時字幕</div>
        <div class="ep-sub">需先下載下列項目（約 {{ gb(prov.state.totalMb) }} GB）：</div>
        <ul class="ep-list">
          <li v-for="m in prov.state.missing" :key="m.kind">
            <span>{{ KIND_LABEL[m.kind] ?? m.kind }}</span>
            <span class="ep-sz">{{ m.sizeMb }} MB</span>
          </li>
        </ul>
        <div class="ep-actions">
          <button class="ep-btn ghost" @click="prov.cancel()">否</button>
          <button class="ep-btn primary" @click="prov.confirm()">是，下載</button>
        </div>
      </template>

      <!-- 下載中（不可關閉） -->
      <template v-else-if="prov.state.phase === 'downloading'">
        <div class="ep-title">正在下載引擎…</div>
        <div class="ep-cur">{{ current ? current.label : '準備中…' }}</div>
        <div class="ep-bar"><div :style="{ width: (current ? current.pct : 0) + '%' }"></div></div>
        <div class="ep-pct">{{ current ? current.text : '' }}</div>
        <div class="ep-note">下載期間請勿關閉視窗。</div>
      </template>

      <!-- 失敗（可關閉） -->
      <template v-else>
        <div class="ep-title">下載失敗</div>
        <div class="ep-err">{{ prov.state.error }}</div>
        <div class="ep-actions">
          <button class="ep-btn ghost" @click="prov.cancel()">關閉</button>
          <button class="ep-btn primary" @click="prov.retry()">重試</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.ep-backdrop { position: fixed; inset: 0; z-index: 60; background: rgba(0,0,0,0.45); display: flex; align-items: center; justify-content: center; }
.ep { width: 380px; max-width: 90vw; background: rgba(26,26,30,0.92); backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4); border: 1px solid rgba(255,255,255,0.12); border-radius: 14px; box-shadow: 0 24px 60px rgba(0,0,0,.65); color: #e8e8ea; padding: 20px; font-size: 13px; }
.ep-title { font-size: 15px; font-weight: 600; color: #fff; margin-bottom: 8px; }
.ep-sub { color: #b8b8bf; margin-bottom: 12px; }
.ep-list { list-style: none; margin: 0 0 16px; padding: 0; }
.ep-list li { display: flex; justify-content: space-between; padding: 7px 0; border-bottom: 1px solid rgba(255,255,255,0.07); }
.ep-sz { color: #8f8f97; }
.ep-cur { color: #cfd8ff; margin-bottom: 8px; }
.ep-bar { height: 6px; background: rgba(255,255,255,0.14); border-radius: 3px; overflow: hidden; margin-bottom: 8px; }
.ep-bar > div { height: 100%; background: var(--accent); border-radius: 3px; transition: width .2s ease; }
.ep-pct { font-size: 11px; color: #9fc6ff; text-align: right; min-height: 14px; }
.ep-note { font-size: 11px; color: #7a7a82; margin-top: 12px; }
.ep-err { color: #ff9a9a; margin-bottom: 16px; word-break: break-word; }
.ep-actions { display: flex; justify-content: flex-end; gap: 8px; }
.ep-btn { padding: 7px 16px; border-radius: 8px; border: none; cursor: pointer; font-size: 13px; }
.ep-btn.ghost { background: rgba(255,255,255,0.1); color: #d8d8de; }
.ep-btn.primary { background: var(--accent); color: #fff; }
</style>
