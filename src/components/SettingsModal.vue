<script setup lang="ts">
import { ref } from 'vue'
import { useSettings } from '../player/useSettings'
import SettingsIcon from './settings/SettingsIcon.vue'
import LiveSubsPanel from './settings/LiveSubsPanel.vue'
import TranslatePanel from './settings/TranslatePanel.vue'
import HardwarePanel from './settings/HardwarePanel.vue'
import SubtitleStylePanel from './settings/SubtitleStylePanel.vue'
import CapturePanel from './settings/CapturePanel.vue'
import PlaceholderPanel from './settings/PlaceholderPanel.vue'

const settings = useSettings()
const current = ref<string>('liveSubs')

const GENERAL = [
  { key: 'ui', label: '介面', icon: 'app-window' },
  { key: 'playback', label: '播放', icon: 'play' },
  { key: 'video', label: '影像', icon: 'monitor' },
  { key: 'audio', label: '音訊', icon: 'volume' },
  { key: 'hardware', label: '硬體加速', icon: 'cpu' },
  { key: 'appearance', label: '字幕外觀', icon: 'type' },
]
const ADVANCED = [
  { key: 'liveSubs', label: '即時字幕', icon: 'captions' },
  { key: 'translate', label: '字幕翻譯', icon: 'translate' },
  { key: 'capture', label: '擷取與錄製', icon: 'video' },
  { key: 'network', label: '網路', icon: 'globe' },
]
const PH = {
  ui: { title: '介面', icon: 'app-window' },
  playback: { title: '播放', icon: 'play' },
  video: { title: '影像', icon: 'monitor' },
  audio: { title: '音訊', icon: 'volume' },
  capture: { title: '擷取與錄製', icon: 'video' },
  network: { title: '網路', icon: 'globe' },
} as Record<string, { title: string; icon: string }>
</script>

<template>
  <div v-if="settings.modal.open" class="sm-backdrop" @pointerdown.stop @click.self="settings.closeModal()">
    <div class="sm">
      <div class="sm-head"><h3>設定</h3><button class="sm-x" aria-label="關閉" @click="settings.closeModal()">✕</button></div>
      <div class="sm-body">
        <nav class="sm-nav">
          <div class="sm-nav-g">一般</div>
          <button v-for="it in GENERAL" :key="it.key" class="sm-nav-i" :class="{ sel: current === it.key }" @click="current = it.key">
            <SettingsIcon :name="it.icon" /> {{ it.label }}
          </button>
          <div class="sm-nav-g">進階</div>
          <button v-for="it in ADVANCED" :key="it.key" class="sm-nav-i" :class="{ sel: current === it.key }" @click="current = it.key">
            <SettingsIcon :name="it.icon" /> {{ it.label }}
          </button>
        </nav>
        <div class="sm-content">
          <LiveSubsPanel v-if="current === 'liveSubs'" />
          <TranslatePanel v-else-if="current === 'translate'" />
          <HardwarePanel v-else-if="current === 'hardware'" />
          <SubtitleStylePanel v-else-if="current === 'appearance'" />
          <CapturePanel v-else-if="current === 'capture'" />
          <PlaceholderPanel v-else :title="PH[current].title" :icon="PH[current].icon" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sm-backdrop { position: fixed; inset: 0; z-index: 50; background: rgba(0,0,0,0.28); display: flex; align-items: center; justify-content: center; }
.sm { width: 760px; max-width: 92vw; height: 520px; max-height: 88vh; background: rgba(26,26,30,0.85); backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4); border: 1px solid rgba(255,255,255,0.1); border-radius: 14px; box-shadow: 0 24px 60px rgba(0,0,0,.6); overflow: hidden; color: #e8e8ea; font-size: 13px; display: flex; flex-direction: column; }
.sm-head { display: flex; align-items: center; justify-content: space-between; padding: 13px 16px; border-bottom: 1px solid rgba(255,255,255,0.08); }
.sm-head h3 { margin: 0; font-size: 14px; font-weight: 600; color: #fff; }
.sm-x { background: none; border: none; color: #9a9aa0; cursor: pointer; font-size: 15px; }
.sm-body { display: flex; flex: 1; min-height: 0; }
.sm-nav { width: 178px; background: rgba(0,0,0,0.18); border-right: 1px solid rgba(255,255,255,0.07); padding: 10px 8px; display: flex; flex-direction: column; gap: 1px; overflow: auto; }
.sm-nav-i { display: flex; align-items: center; gap: 10px; padding: 7px 11px; border-radius: 7px; color: #c2c2c8; cursor: pointer; background: none; border: none; text-align: left; font-size: 13px; }
.sm-nav-i:hover { background: rgba(255,255,255,0.07); }
.sm-nav-i.sel { background: var(--accent); color: #fff; }
.sm-nav-g { font-size: 10px; letter-spacing: .5px; color: #6b6b73; padding: 9px 11px 3px; text-transform: uppercase; }
.sm-content { flex: 1; padding: 18px 20px; overflow: auto; scrollbar-gutter: stable; }

/* 細捲軸，搭深色玻璃 */
.sm-nav::-webkit-scrollbar,
.sm-content::-webkit-scrollbar { width: 10px; }
.sm-nav::-webkit-scrollbar-track,
.sm-content::-webkit-scrollbar-track { background: transparent; }
.sm-nav::-webkit-scrollbar-thumb,
.sm-content::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.16);
  border-radius: 6px;
  border: 2px solid transparent;
  background-clip: padding-box;
}
.sm-nav::-webkit-scrollbar-thumb:hover,
.sm-content::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
  background-clip: padding-box;
}
</style>
