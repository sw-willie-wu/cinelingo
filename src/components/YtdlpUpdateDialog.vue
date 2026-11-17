<script setup lang="ts">
import { computed } from 'vue'
import { useYtdlpUpdate } from '../player/useYtdlpUpdate'

const yu = useYtdlpUpdate()

const progressText = computed(() =>
  yu.state.totalMb > 0 ? `${yu.state.pct}% · ${yu.state.doneMb} / ${yu.state.totalMb} MB` : '準備中…')
</script>

<template>
  <div v-if="yu.state.open" class="ep-backdrop">
    <div class="ep">
      <template v-if="yu.state.phase === 'ask'">
        <div class="ep-title">YouTube 解析元件有新版</div>
        <div class="ep-sub">yt-dlp {{ yu.state.local }} → {{ yu.state.latest }}。舊版可能導致 YouTube 無法播放。</div>
        <div class="ep-actions">
          <button class="ep-btn ghost" @click="yu.later()">稍後</button>
          <button class="ep-btn primary" @click="yu.update()">更新</button>
        </div>
      </template>
      <template v-else-if="yu.state.phase === 'downloading'">
        <div class="ep-title">正在更新 yt-dlp…</div>
        <div class="ep-bar"><div :style="{ width: yu.state.pct + '%' }"></div></div>
        <div class="ep-pct">{{ progressText }}</div>
        <div class="ep-note">下載期間請勿關閉視窗。</div>
      </template>
      <template v-else-if="yu.state.phase === 'done'">
        <div class="ep-title">已更新至 {{ yu.state.version }}</div>
        <div class="ep-actions">
          <button class="ep-btn primary" @click="yu.close()">關閉</button>
        </div>
      </template>
      <template v-else>
        <div class="ep-title">更新失敗</div>
        <div class="ep-err">{{ yu.state.error }}</div>
        <div class="ep-actions">
          <button class="ep-btn ghost" @click="yu.close()">關閉</button>
          <button class="ep-btn primary" @click="yu.update()">重試</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.ep-backdrop { position: fixed; inset: 0; z-index: 60; background: rgba(0,0,0,0.45); display: flex; align-items: center; justify-content: center; }
.ep { width: 380px; max-width: 90vw; background: rgba(26,26,30,0.92); backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4); border: 1px solid rgba(255,255,255,0.12); border-radius: 14px; box-shadow: 0 24px 60px rgba(0,0,0,.65); color: #e8e8ea; padding: 20px; font-size: 13px; }
.ep-title { font-size: 15px; font-weight: 600; color: #fff; margin-bottom: 8px; }
.ep-sub { color: #b8b8bf; margin-bottom: 16px; }
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
