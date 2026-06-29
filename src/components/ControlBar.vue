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
import MicButton from './MicButton.vue'
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
// 只有「真的在播」(非 idle 且未暫停)才顯示暫停 icon；idle(含停止後 pause 仍為 false)一律顯示播放。
const isPlaying = computed(() => !player.isIdle.value && player.state.pause === false)
</script>

<template>
  <div class="control-bar">
    <SeekBar v-if="player.state.path" />
    <div class="osc-row">
      <!-- 左:音量 + 時間（慣例上左側較輕，平衡靠分群與留白而非等量） -->
      <div class="group left">
        <VolumeControl />
        <span v-if="player.state.path" class="time">{{ timeText }}</span>
      </div>

      <!-- 中:prev / 後退5 / 播放 / 前進5 / next(絕對置中、純傳輸) -->
      <div class="group center">
        <button class="btn" aria-label="上一個" @click="queue.prev()"><PlayerIcon name="prev" /></button>
        <button class="btn" aria-label="後退5秒" @click="player.seekBy(-5)"><PlayerIcon name="back5" /></button>
        <button class="btn btn-play" :aria-label="isPlaying ? '暫停' : '播放'" @click="player.togglePause()">
          <PlayerIcon :name="isPlaying ? 'pause' : 'play'" :size="22" />
        </button>
        <button class="btn" aria-label="前進5秒" @click="player.seekBy(5)"><PlayerIcon name="forward5" /></button>
        <button class="btn" aria-label="下一個" @click="queue.next()"><PlayerIcon name="next" /></button>
      </div>

      <!-- 右:字幕/AI 藥丸 + 置頂藥丸 + 全螢幕 + 清單 + 更多 -->
      <div class="group right">
        <!-- 字幕/AI:CC + 外部音源 -->
        <div class="pill" role="group" aria-label="字幕與音訊">
          <SubtitleControls />
          <MicButton />
        </div>

        <!-- 視窗/檢視:浮動字幕 + 釘選最上層 + 全螢幕 -->
        <div class="pill" role="group" aria-label="視窗與檢視">
          <button class="btn" aria-label="浮動字幕" title="浮動字幕（疊在其他視窗上）" @click="floating.enter()">
            <PlayerIcon name="floating-subs" />
          </button>
          <button class="btn" :class="{ on: alwaysOnTop }" aria-label="釘選最上層" title="釘選最上層" @click="toggleAlwaysOnTop()">
            <PlayerIcon name="pip" />
          </button>
          <button class="btn" :aria-label="player.state.fullscreen ? '退出全螢幕' : '全螢幕'" @click="player.toggleFullscreen()">
            <PlayerIcon :name="player.state.fullscreen ? 'fullscreen-exit' : 'fullscreen'" />
          </button>
        </div>

        <!-- 待播清單:放在更多左邊（側欄從右側展開） -->
        <button class="btn" :class="{ on: queue.items.length > 1 }" aria-label="待播清單" title="待播清單" @click="showQueue = !showQueue">
          <PlayerIcon name="playlist" />
        </button>

        <!-- 更多/調整:popover 固定從右下角彈出,放最右與其對齊 -->
        <button class="btn adjust-btn" aria-label="更多設定" title="更多設定" @click="showAdjust = !showAdjust">
          <PlayerIcon name="more" />
          <span v-if="qualityBadge" class="q-badge">{{ qualityBadge }}</span>
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
/* 右群組:藥丸之間留較大間距,讓眼睛把每個藥丸讀成一個群 */
.group.right { gap: 8px; }
/* segmented toggle group:共用背景圈出「相關功能成一團」
   上下也要留 padding,否則滿高按鈕會蓋掉填色、框就看不出來 */
.pill {
  display: flex; align-items: center; gap: 2px;
  padding: 1px 4px;
  background: rgba(255, 255, 255, 0.12);
  border-radius: 999px;
}
.time { color: #fff; font: 12px/1 var(--font); white-space: nowrap; user-select: none; margin-left: 12px; }
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
  background: var(--accent); color: #fff; font: 700 8px/1 var(--font); letter-spacing: .2px;
  border-radius: 7px; pointer-events: none;
}
.btn-play {
  width: 48px; height: 48px; border-radius: 14px; color: rgba(255, 255, 255, 0.96);
}
.btn-play:hover { color: #fff; filter: drop-shadow(0 0 5px rgba(255, 255, 255, 0.8)) drop-shadow(0 0 15px rgba(255, 255, 255, 0.5)); }
.btn-play:active { transform: scale(0.92); }
</style>
