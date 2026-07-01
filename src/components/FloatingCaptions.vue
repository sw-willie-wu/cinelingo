<script setup lang="ts">
import { computed, ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useSubtitles } from '../player/useSubtitles'
import { useSettings } from '../player/useSettings'
import { useFloatingMode } from '../player/useFloatingMode'
import { usePlayer } from '../player/usePlayer'
import { subTextStyle, scaleFontPx } from '../player/settings'
import { liveLines, liveBlocks, displayCharCap } from '../player/subtitles'

const subs = useSubtitles()
const settings = useSettings()
const floating = useFloatingMode()
const player = usePlayer()
const win = getCurrentWebviewWindow()

// 浮動條字體＝「全螢幕時的大小」：用螢幕高當 scaleFontPx 基準（≡ 影片 overlay 全螢幕的字級），
// 不隨那條視窗的高縮放。4K 螢幕上設定 28 → ~56px，與全螢幕看影片時一致。
const FONT_REF_H = window.screen.height || 1080
const winW = ref(window.innerWidth)
const onResize = () => { winW.value = window.innerWidth }
onMounted(() => window.addEventListener('resize', onResize))
onBeforeUnmount(() => window.removeEventListener('resize', onResize))

const style = computed(() => settings.state.appearance.primary)
const fontPx = computed(() => scaleFontPx(style.value.fontSize, FONT_REF_H))
// 底色/外框/顏色完全跟「字幕外觀（主字幕）」走，不自己加保底底襯（想要底色→設定→字幕外觀→背景設半透明）。
const textStyle = computed(() => subTextStyle(style.value, FONT_REF_H))
const cap = computed(() =>
  displayCharCap(winW.value, fontPx.value, settings.state.appearance.maxWidthPct / 100)
)

// 兩種字幕來源：
//  - loopback（noClock）＝外部內容，多行 final + interim 串流
//  - 時鐘字幕（沿用既有：字幕檔 / mode A 即時字幕）＝隨播放時間 time-pos 取當下該顯示的句子
const isLoopback = computed(() => subs.noClock.value)
const t = computed(() => player.state.timePos ?? 0)
const live = computed(() =>
  isLoopback.value
    ? liveLines(subs.liveCues.value, subs.liveInterim.value, settings.state.liveSubs.display.lines, cap.value)
    : { lines: [] as string[], interimLines: [] as string[] }
)
const translateOn = computed(() => settings.state.liveSubs.translateEnabled)
const blocks = computed(() =>
  isLoopback.value && translateOn.value
    ? liveBlocks(subs.liveCues.value, subs.liveInterim.value, settings.state.liveSubs.display.lines, cap.value)
    : []
)
const secStyle = computed(() => subTextStyle(settings.state.appearance.secondary, FONT_REF_H))
const clockText = computed(() => (isLoopback.value ? '' : subs.activeText('primary', t.value)))
const clockTranscribing = computed(() => !isLoopback.value && subs.isTranscribing('primary', t.value))

// 閒置偵測（僅 loopback 用）：超過 IDLE_MS 沒新字幕 → 顯示待命提示讓使用者找得到字幕條。
// 時鐘字幕模式的句間空檔屬正常，不顯示待命提示（避免安靜片段一直閃）。
const IDLE_MS = 4000
const hasContent = computed(() =>
  isLoopback.value
    ? (translateOn.value ? blocks.value.length > 0 : (live.value.lines.length > 0 || live.value.interimLines.length > 0))
    : !!clockText.value
)
const now = ref(Date.now())
let lastActivity = Date.now()
let idleTimer: ReturnType<typeof setInterval> | undefined
onMounted(() => { idleTimer = setInterval(() => { now.value = Date.now() }, 1000) })
onBeforeUnmount(() => { if (idleTimer) clearInterval(idleTimer) })
watch(live, () => { lastActivity = Date.now() })   // loopback 內容變動＝活動
const idle = computed(() => now.value - lastActivity > IDLE_MS)
const showCaption = computed(() => (isLoopback.value ? hasContent.value && !idle.value : hasContent.value))

// 拖曳/調寬期間維持灰底+控件顯示：OS 拖曳會讓 webview 收到 mouseleave → hovering 變 false，
// 用 dragging 旗標補住。⚠️ OS 拖曳後 webview 常收不到 mouseup → 不能只靠 mouseup 清；
// 改用「滑鼠移動且未按任何鍵(buttons===0)就清」當主要清除（放開拖曳後一動就清），mouseup 為輔。
const dragging = ref(false)
const showOverlay = computed(() => floating.hovering.value || dragging.value)
const endDrag = () => { dragging.value = false }
const onGlobalMove = (e: MouseEvent) => { if (e.buttons === 0) dragging.value = false }
onMounted(() => { window.addEventListener('mouseup', endDrag); window.addEventListener('mousemove', onGlobalMove) })
onBeforeUnmount(() => { window.removeEventListener('mouseup', endDrag); window.removeEventListener('mousemove', onGlobalMove) })

// 任意位置拖曳：mousedown 在條上任一處 → 移動視窗；退出鈕/調寬邊除外（各自處理）。
function onRootDown(e: MouseEvent) {
  if (e.button !== 0) return
  const el = e.target as HTMLElement | null
  if (el?.closest('.ctl-btn, .resize-edge')) return
  dragging.value = true
  win.startDragging()
}
function onResizeWest() { dragging.value = true; win.startResizeDragging('West') }
function onResizeEast() { dragging.value = true; win.startResizeDragging('East') }
</script>

<template>
  <div
    class="floating-root"
    @mouseenter="floating.hovering.value = true"
    @mouseleave="floating.hovering.value = false"
    @mousedown="onRootDown"
  >
    <!-- hover 時的半透明底（放字幕下層，不遮字幕）；與控件一起淡入 -->
    <Transition name="ctl">
      <div v-show="showOverlay" class="hover-bg"></div>
    </Transition>
    <div class="cap-area">
      <!-- loopback（外部內容）：多行 final + interim -->
      <span v-if="isLoopback && showCaption" class="cap-text" :style="textStyle">
        <!-- translate 開：逐 block（原文行 + 譯文行） -->
        <template v-if="translateOn">
          <template v-for="b in blocks" :key="b.id">
            <span :class="{ interim: b.interim }">{{ b.sourceLines.join('\n') }}</span>
            <span v-if="b.target" class="xlate" :style="secStyle">{{ '\n' + b.target }}</span>
            <span>{{ '\n' }}</span>
          </template>
        </template>
        <!-- translate 關：既有扁平 lines（位元不變） -->
        <template v-else
          ><span v-if="live.lines.length">{{ live.lines.join('\n') }}</span
          ><span v-if="live.interimLines.length" class="interim">{{ (live.lines.length ? '\n' : '') + live.interimLines.join('\n') }}</span></template
        >
      </span>
      <!-- 時鐘字幕（沿用既有：字幕檔 / mode A 即時字幕）：當下該句 -->
      <span v-else-if="!isLoopback && showCaption" class="cap-text" :style="textStyle">{{ clockText }}</span>
      <!-- mode A 即時字幕還沒追上 -->
      <span v-else-if="clockTranscribing" class="cap-text dim" :style="textStyle">（轉錄中…）</span>
      <!-- loopback 待命提示（時鐘字幕的句間空檔則留白，不顯示） -->
      <span v-else-if="isLoopback" class="idle-hint">🎧 即時字幕待命中…</span>
    </div>

    <Transition name="ctl">
      <div v-show="showOverlay" class="ctl-layer">
        <button class="ctl-btn exit" aria-label="退出浮動字幕" title="退出浮動字幕" @click="floating.exit()">✕</button>
        <div class="resize-edge west" @mousedown="onResizeWest"></div>
        <div class="resize-edge east" @mousedown="onResizeEast"></div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.floating-root { position: fixed; inset: 0; cursor: move; }   /* 任意位置可拖曳 */
.cap-area { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; padding: 0 24px; }
.cap-text { line-height: 1.3; text-align: center; white-space: pre-wrap; }
.cap-text .interim { opacity: 0.75; }
.cap-text.dim { opacity: 0.55; }
/* 閒置提示：淡淡的小膠囊，讓使用者找得到字幕條（無字幕時才顯示）。 */
.idle-hint {
  font-size: 13px; color: rgba(255,255,255,0.72); background: rgba(0,0,0,0.42);
  padding: 3px 12px; border-radius: 7px; user-select: none; white-space: nowrap;
}
/* hover 半透明底：放字幕下層（floating-root 第一個子元素），讓使用者找得到/抓得到視窗、又不遮字幕。 */
.hover-bg { position: absolute; inset: 0; pointer-events: none; background: rgba(0,0,0,0.38); border-radius: 8px; }
.ctl-layer { position: absolute; inset: 0; pointer-events: none; }
.ctl-layer > * { pointer-events: auto; }
.ctl-btn.exit {
  position: absolute; top: 4px; right: 6px; width: 24px; height: 24px;
  background: rgba(0,0,0,0.5); color: #fff; border: none; border-radius: 6px; cursor: pointer; font-size: 13px;
}
.ctl-btn.exit:hover { background: rgba(220,60,60,0.7); }
.resize-edge { position: absolute; top: 0; bottom: 0; width: 8px; cursor: ew-resize; }
.resize-edge.west { left: 0; } .resize-edge.east { right: 0; }
.ctl-enter-active, .ctl-leave-active { transition: opacity 0.18s ease; }
.ctl-enter-from, .ctl-leave-to { opacity: 0; }
</style>
