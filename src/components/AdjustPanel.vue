<script setup lang="ts">
import { ref, computed } from 'vue'
import { usePlayer } from '../player/usePlayer'
import { useSpeed } from '../player/useSpeed'
import { useSettings } from '../player/useSettings'
import { useSubtitles } from '../player/useSubtitles'
import { useAudioAdjust } from '../player/useAudioAdjust'
import { useVideoAdjust } from '../player/useVideoAdjust'
import type { ImageProp } from '../mpv'
import SpeedScreen from './adjust/SpeedScreen.vue'
import QualityScreen from './adjust/QualityScreen.vue'
import ImageScreen from './adjust/ImageScreen.vue'
import EqScreen from './adjust/EqScreen.vue'
import NormalizeScreen from './adjust/NormalizeScreen.vue'
import AudioDelayScreen from './adjust/AudioDelayScreen.vue'
import SubtitleTrackScreen from './adjust/SubtitleTrackScreen.vue'

defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()
type Screen = 'root' | 'speed' | 'quality' | 'image' | 'eq' | 'normalize' | 'audiodelay' | 'primary' | 'secondary'
const screen = ref<Screen>('root')
const player = usePlayer()
const { speed } = useSpeed()
const settings = useSettings()
const subs = useSubtitles()
const audio = useAudioAdjust()
const video = useVideoAdjust()
const isRemote = computed(() => player.source.current?.kind === 'remote')
const qualityLabel = computed(() => settings.state.youtube.quality === 'auto' ? '自動' : `${settings.state.youtube.quality}p`)
const secondaryDisabled = computed(() => subs.tracks.primary.source === 'off')

// 第一層現值摘要（spec §2.3）
const IMG_ABBR: [ImageProp, string][] = [['brightness', '亮'], ['contrast', '對'], ['saturation', '飽'], ['gamma', 'G'], ['hue', '色']]
const imageLabel = computed(() => {
  const parts = IMG_ABBR.filter(([k]) => video.state[k] !== 0).map(([k, l]) => `${l}${video.state[k] > 0 ? '+' : ''}${video.state[k]}`)
  if (video.state.deband) parts.push('去色帶')
  return parts.length ? parts.join('・') : '預設'
})
function trackLabel(track: 'primary' | 'secondary'): string {
  const s = subs.tracks[track].source
  if (s === 'off') return '關'
  if (s === 'live') return '即時'
  return subs.files.value.find((f) => f.id === s)?.name ?? '字幕檔'
}

function go(s: Screen) { screen.value = s }
function close() { screen.value = 'root'; emit('close') }
</script>

<template>
  <div v-if="open" class="sm-pop" @click.self="close">
    <div class="sm-menu" @click.stop>
      <template v-if="screen === 'root'">
        <div class="sec">播放</div>
        <div class="item" @click="go('speed')"><span class="ic">⏩</span><span class="nm">播放速度</span><span class="cur">{{ speed }}×</span><span class="gt">›</span></div>
        <div class="item" :class="{ disabled: !isRemote }" @click="isRemote && go('quality')"><span class="ic">▢</span><span class="nm">畫質</span><span class="cur">{{ isRemote ? qualityLabel : '僅串流影片' }}</span><span class="gt">›</span></div>
        <div class="sec">影像</div>
        <div class="item" @click="go('image')"><span class="ic">◐</span><span class="nm">影像調整</span><span class="cur">{{ imageLabel }}</span><span class="gt">›</span></div>
        <div class="sec">音訊</div>
        <div class="item" @click="go('eq')"><span class="ic">🎚</span><span class="nm">等化器 EQ</span><span class="cur">{{ audio.eq.enabled ? audio.eq.preset : '關' }}</span><span class="gt">›</span></div>
        <div class="item" @click="go('normalize')"><span class="ic">◎</span><span class="nm">音量正規化</span><span class="cur">{{ audio.normalize() ? '開' : '關' }}</span><span class="gt">›</span></div>
        <div class="item" @click="go('audiodelay')"><span class="ic">⇄</span><span class="nm">音訊延遲</span><span class="cur">{{ audio.audioDelayMs.value }}ms</span><span class="gt">›</span></div>
        <div class="sec">字幕</div>
        <div class="item" @click="go('primary')"><span class="ic">字</span><span class="nm">主字幕</span><span class="cur">{{ trackLabel('primary') }}</span><span class="gt">›</span></div>
        <div class="item" :class="{ disabled: secondaryDisabled }" @click="!secondaryDisabled && go('secondary')">
          <span class="ic">字</span><span class="nm">第二字幕</span><span class="cur">{{ secondaryDisabled ? '需先開主字幕' : trackLabel('secondary') }}</span><span class="gt">›</span>
        </div>
      </template>
      <SpeedScreen v-else-if="screen === 'speed'" @back="go('root')" />
      <QualityScreen v-else-if="screen === 'quality'" @back="go('root')" />
      <ImageScreen v-else-if="screen === 'image'" @back="go('root')" />
      <EqScreen v-else-if="screen === 'eq'" @back="go('root')" />
      <NormalizeScreen v-else-if="screen === 'normalize'" @back="go('root')" />
      <AudioDelayScreen v-else-if="screen === 'audiodelay'" @back="go('root')" />
      <SubtitleTrackScreen v-else-if="screen === 'primary'" track="primary" @back="go('root')" />
      <SubtitleTrackScreen v-else-if="screen === 'secondary'" track="secondary" @back="go('root')" />
    </div>
  </div>
</template>

<style scoped>
.sm-pop { position: fixed; inset: 0; z-index: 40; }
.sm-menu { position: absolute; right: 14px; bottom: 60px; width: 312px;
  background: rgba(24,25,30,0.92); backdrop-filter: blur(26px) saturate(1.4); -webkit-backdrop-filter: blur(26px) saturate(1.4);
  border: 1px solid rgba(255,255,255,0.12); border-radius: 14px; box-shadow: 0 20px 50px rgba(0,0,0,.6); color: #e8e8ea; font-size: 13px; overflow: hidden; }
.sec { font-size: 10px; letter-spacing: .5px; text-transform: uppercase; color: #6b6e7a; padding: 11px 15px 5px; }
.item { display: flex; align-items: center; gap: 12px; padding: 11px 15px; cursor: pointer; }
.item:hover { background: rgba(255,255,255,0.04); }
.item.disabled { opacity: .4; cursor: not-allowed; }
.item .ic { font-size: 15px; width: 20px; text-align: center; color: #cfd2db; }
.item .nm { flex: 1; white-space: nowrap; } .item .cur { color: #8a8d99; font-size: 12px; max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; } .item .gt { color: #6b6e7a; margin-left: 4px; }
</style>
