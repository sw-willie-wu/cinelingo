<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { useSubtitles, type TrackName } from '../player/useSubtitles'
import { usePlayer } from '../player/usePlayer'
import { useSettings } from '../player/useSettings'
import { subTextStyle, scaleFontPx } from '../player/settings'
import { liveLines, displayCharCap } from '../player/subtitles'

const subs = useSubtitles()
const player = usePlayer()
const settings = useSettings()
const t = computed(() => player.state.timePos ?? 0)
const winH = ref(window.innerHeight)
const winW = ref(window.innerWidth)
const onResize = () => { winH.value = window.innerHeight; winW.value = window.innerWidth }
onMounted(() => window.addEventListener('resize', onResize))
onBeforeUnmount(() => window.removeEventListener('resize', onResize))

function line(track: TrackName, styleKey: 'primary' | 'secondary') {
  const style = computed(() => settings.state.appearance[styleKey])
  return {
    text: computed(() => subs.activeText(track, t.value)),
    transcribing: computed(() => subs.isTranscribing(track, t.value)),
    textStyle: computed(() => subTextStyle(style.value, winH.value)),
    dimStyle: computed(() => ({ ...subTextStyle(style.value, winH.value), fontSize: `${scaleFontPx(style.value.fontSize * 0.7, winH.value)}px` })),
    overlayStyle: computed(() => ({ bottom: `${style.value.bottomPct}%` })),
  }
}
const p = line('primary', 'primary')
const s = line('secondary', 'secondary')
// 即時字幕一行字數上限：依視窗寬 × 主字幕字型 px × 使用者設定的寬度% 動態算。
const liveCap = computed(() =>
  displayCharCap(
    winW.value,
    scaleFontPx(settings.state.appearance.primary.fontSize, winH.value),
    settings.state.appearance.maxWidthPct / 100,
  )
)
// clock-mode 字幕一行最大寬度＝視窗寬 × maxWidthPct%（與 loopback「佔視窗寬比例」同語意）。
const clockMaxWidthStyle = computed(() => ({
  maxWidth: `${Math.round(winW.value * settings.state.appearance.maxWidthPct / 100)}px`,
}))
const live = computed((): { lines: string[]; interimLines: string[] } =>
  subs.noClock.value
    ? liveLines(subs.liveCues.value, subs.liveInterim.value, settings.state.liveSubs.display.lines, liveCap.value)
    : { lines: [] as string[], interimLines: [] as string[] }
)
</script>

<template>
  <!-- no-clock (loopback)：多行 last-N final + interim（較淡） -->
  <div
    v-if="subs.noClock.value && (live.lines.length || live.interimLines.length)"
    class="sub-overlay"
    :style="p.overlayStyle.value"
  >
    <span class="sub-text" :style="p.textStyle.value"
      ><span v-if="live.lines.length">{{ live.lines.join('\n') }}</span
      ><span v-if="live.interimLines.length" class="interim">{{ (live.lines.length ? '\n' : '') + live.interimLines.join('\n') }}</span></span
    >
  </div>

  <!-- 時鐘模式（檔案/YT 播放）：維持原雙軌 -->
  <template v-if="!subs.noClock.value">
    <div v-if="p.text.value || p.transcribing.value" class="sub-overlay" :style="p.overlayStyle.value">
      <span v-if="p.text.value" class="sub-text" :style="[p.textStyle.value, clockMaxWidthStyle]">{{ p.text.value }}</span>
      <span v-else class="sub-text dim" :style="p.dimStyle.value">(轉錄中)</span>
    </div>
    <div v-if="s.text.value || s.transcribing.value" class="sub-overlay" :style="s.overlayStyle.value">
      <span v-if="s.text.value" class="sub-text" :style="[s.textStyle.value, clockMaxWidthStyle]">{{ s.text.value }}</span>
      <span v-else class="sub-text dim" :style="s.dimStyle.value">(轉錄中)</span>
    </div>
  </template>
</template>

<style scoped>
.sub-overlay { position: absolute; left: 0; right: 0; display: flex; justify-content: center; pointer-events: none; padding: 0 5%; z-index: 5; }
.sub-text { line-height: 1.3; text-align: center; white-space: pre-wrap; }
.sub-text.dim { opacity: 0.6; }
.sub-text .interim { opacity: 0.75; }
</style>
