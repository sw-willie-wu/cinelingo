<script setup lang="ts">
import { computed, ref } from 'vue'
import { usePlayer } from '../player/usePlayer'
import { useQueue } from '../player/useQueue'
import { formatClock } from '../player/format'
import { useWindowControls } from '../player/useWindowControls'
import { useFloatingMode } from '../player/useFloatingMode'
import PlayerIcon from './PlayerIcon.vue'
import SeekBar from './SeekBar.vue'
import VolumeControl from './VolumeControl.vue'
import SubtitleControls from './SubtitleControls.vue'
import QueuePanel from './QueuePanel.vue'
import AdjustPanel from './AdjustPanel.vue'
import { useSettings } from '../player/useSettings'
import { pickForPref } from '../player/quality'

const player = usePlayer()
const settings = useSettings()
// 調整鈕的當前解析度 badge：僅遠端（畫質可選）時顯示；數字依實際挑到的影軌（auto 已套螢幕上限）。
const qualityBadge = computed(() => {
  const cur = player.source.current
  if (!cur || cur.kind !== 'remote') return ''
  const v = pickForPref(cur.resolved.videos, settings.state.youtube.quality)
  if (!v) return ''
  if (v.height >= 2160) return '4K'
  if (v.height >= 1440) return '2K'
  if (v.height >= 1080) return 'HD'
  return ''   // 1080 以下不顯示 badge
})
const queue = useQueue()   // 同時觸發 ensureWired → app 啟動即註冊 end-file/file-loaded/start-file listener
const { alwaysOnTop, toggleAlwaysOnTop } = useWindowControls()
const floating = useFloatingMode()
const timeText = computed(() => {
  const qs = player.source.qualitySwitch
  if (qs.active) return `${formatClock(qs.posSec)} / ${formatClock(qs.durSec)}`  // 切畫質期間凍結
  return `${formatClock(player.state.timePos)} / ${formatClock(player.state.duration)}`
})
const showQueue = ref(false)
const showAdjust = ref(false)
</script>

<template>
  <div class="control-bar">
    <SeekBar v-if="player.state.path" />
    <div class="osc-row">
      <!-- 左:音量 + 時間 -->
      <div class="group left">
        <VolumeControl />
        <span v-if="player.state.path" class="time">{{ timeText }}</span>
      </div>

      <!-- 中:prev / 後退5 / 停止 / 播放 / 前進5 / next(絕對置中) -->
      <div class="group center">
        <button class="btn" aria-label="上一個" @click="queue.prev()"><PlayerIcon name="prev" /></button>
        <button class="btn" aria-label="後退5秒" @click="player.seekBy(-5)"><PlayerIcon name="back5" /></button>
        <button class="btn stop-btn" :disabled="player.isIdle.value" aria-label="停止播放" title="停止播放（回首頁）" @click="player.closeMedia()">
          <span class="stop-sq" />
        </button>
        <button class="btn btn-play" :aria-label="player.state.pause === false ? '暫停' : '播放'" @click="player.togglePause()">
          <PlayerIcon :name="player.state.pause === false ? 'pause' : 'play'" :size="22" />
        </button>
        <button class="btn" aria-label="前進5秒" @click="player.seekBy(5)"><PlayerIcon name="forward5" /></button>
        <button class="btn" aria-label="下一個" @click="queue.next()"><PlayerIcon name="next" /></button>
      </div>

      <!-- 右:AI 字幕 / 浮動字幕 / 待播清單 / 釘選最上層 / 全螢幕 -->
      <div class="group right">
        <SubtitleControls />
        <button class="btn adjust-btn" aria-label="調整" title="調整" @click="showAdjust = !showAdjust">
          <PlayerIcon name="adjust" />
          <span v-if="qualityBadge" class="q-badge">{{ qualityBadge }}</span>
        </button>
        <button class="btn" aria-label="浮動字幕" title="浮動字幕（疊在其他視窗上）" @click="floating.enter()">
          <PlayerIcon name="floating-subs" />
        </button>
        <button class="btn" :class="{ on: queue.items.length > 1 }" aria-label="待播清單" title="待播清單" @click="showQueue = !showQueue">
          <PlayerIcon name="playlist" />
        </button>
        <button class="btn" :class="{ on: alwaysOnTop }" aria-label="釘選最上層" @click="toggleAlwaysOnTop()">
          <PlayerIcon name="pip" />
        </button>
        <button class="btn" :aria-label="player.state.fullscreen ? '退出全螢幕' : '全螢幕'" @click="player.toggleFullscreen()">
          <PlayerIcon :name="player.state.fullscreen ? 'fullscreen-exit' : 'fullscreen'" />
        </button>
      </div>
    </div>
    <QueuePanel :open="showQueue" @close="showQueue = false" />
    <AdjustPanel :open="showAdjust" @close="showAdjust = false" />
  </div>
</template>

<style scoped>
.control-bar {
  display: flex; flex-direction: column; gap: 6px;
  padding: 8px 14px 10px;
  color: #fff;
}
.osc-row {
  position: relative; display: flex; align-items: center; justify-content: space-between;
  min-height: 46px;
}
.group { display: flex; align-items: center; gap: 5px; }
.time { color: #fff; font: 12px/1 sans-serif; white-space: nowrap; user-select: none; margin-left: 12px; }
.center {
  position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); gap: 6px;
}
.btn {
  width: 40px; height: 40px; display: flex; align-items: center; justify-content: center;
  border-radius: 11px; border: none; background: transparent; color: rgba(255, 255, 255, 0.88);
  cursor: pointer; padding: 0; transition: color 0.18s, filter 0.18s, transform 0.12s;
}
.btn:hover { color: #fff; filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.7)) drop-shadow(0 0 11px rgba(255, 255, 255, 0.42)); }
.btn.on { color: var(--accent); }
.btn.on:hover { filter: drop-shadow(0 0 4px rgba(var(--accent-rgb), 0.8)) drop-shadow(0 0 12px rgba(var(--accent-rgb), 0.5)); }
.adjust-btn { position: relative; }
.q-badge {
  position: absolute; top: 4px; right: 0; min-width: 14px; height: 13px; padding: 0 3px;
  display: flex; align-items: center; justify-content: center;
  background: var(--accent); color: #fff; font: 700 8px/1 sans-serif; letter-spacing: .2px;
  border-radius: 7px; pointer-events: none;
}
.btn-play {
  width: 48px; height: 48px; border-radius: 14px; color: rgba(255, 255, 255, 0.96);
}
.btn-play:hover { color: #fff; filter: drop-shadow(0 0 5px rgba(255, 255, 255, 0.8)) drop-shadow(0 0 15px rgba(255, 255, 255, 0.5)); }
.btn-play:active { transform: scale(0.92); }
.stop-btn:disabled { opacity: 0.35; cursor: default; }
.stop-sq { width: 13px; height: 13px; border-radius: 2px; background: currentColor; display: block; }
</style>
