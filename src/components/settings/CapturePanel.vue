<script setup lang="ts">
import { useSettings } from '../../player/useSettings'
import { useCaptureProvision } from '../../player/useCaptureProvision'
import GlassToggle from '../GlassToggle.vue'
const settings = useSettings()
const cap = useCaptureProvision()
function onToggle(v: boolean) { if (v) cap.enable(); else cap.disable() }
</script>
<template>
  <div>
    <div class="p-title">擷取與錄製</div>
    <div class="p-sub">啟用後可貼上 YouTube 網址（Ctrl+V）直接在 Cinelingo 串流播放。</div>
    <div class="field">
      <div class="lab">啟用 YouTube／網路來源<small>開啟時會下載解析工具 yt-dlp（約 30 MB）。</small></div>
      <GlassToggle :model-value="settings.state.capture.enabled" @update:model-value="onToggle" />
    </div>
    <div v-if="cap.state.phase === 'downloading'" class="note">下載 yt-dlp… {{ cap.state.pct }}%</div>
    <div v-else-if="cap.state.phase === 'checking'" class="note">檢查中…</div>
    <div v-else-if="cap.state.phase === 'error'" class="note err">下載失敗：{{ cap.state.error }} <button @click="cap.retry()">重試</button></div>
    <div v-else-if="settings.state.capture.enabled" class="note">已就緒——貼上 YouTube 網址即可播放。</div>
  </div>
</template>
<style scoped>
.p-title { font-size: 15px; font-weight: 600; color: #fff; margin: 0 0 3px; }
.p-sub { font-size: 12px; color: #8a8a92; margin: 0 0 12px; }
.field { display: flex; align-items: center; justify-content: space-between; padding: 9px 2px; }
.lab { color: #d8d8de; flex: 1; }
.lab small { display: block; color: #7a7a82; font-size: 11px; margin-top: 1px; }
.note { font-size: 12px; color: #9aa0a6; margin: 8px 2px; }
.note.err { color: #ff8a8a; }
.note button { margin-left: 8px; background: none; border: 1px solid var(--accent); color: var(--accent); border-radius: 6px; padding: 2px 8px; cursor: pointer; }
</style>
