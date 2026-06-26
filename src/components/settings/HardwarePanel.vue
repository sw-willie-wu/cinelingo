<script setup lang="ts">
import { computed } from 'vue'
import { useSettings } from '../../player/useSettings'
import SettingsIcon from './SettingsIcon.vue'
import GlassToggle from '../GlassToggle.vue'
import GlassSelect from '../GlassSelect.vue'

const settings = useSettings()
const voOptions = [
  { value: 'gpu', label: 'gpu（穩定）' },
  { value: 'gpu-next', label: 'gpu-next（畫質）' },
]
const info = computed(() => settings.hw.info)
const backendLabel = computed(() => {
  const b = info.value?.backend
  return b === 'cuda' ? 'CUDA' : b === 'vulkan' ? 'Vulkan' : 'CPU'
})
</script>

<template>
  <div>
    <div class="p-title">硬體加速</div>
    <div class="p-sub">AI 即時字幕需大量運算；偵測到支援的 GPU 時會用它加速。影片硬體解碼另在「播放」。</div>

    <div class="field">
      <div class="lab">啟用硬體加速<small>自動偵測（手動關閉即將推出）</small></div>
      <GlassToggle :model-value="!!settings.state.hardware.accelEnabled" disabled />
    </div>

    <div class="field">
      <div class="lab">影片輸出<small>gpu 較穩；gpu-next 畫質較好但偶發 HDR 凍住。重啟後生效</small></div>
      <GlassSelect :model-value="settings.state.playback.videoOutput" :options="voOptions"
        @update:model-value="settings.state.playback.videoOutput = $event as 'gpu' | 'gpu-next'" />
    </div>

    <div v-if="info?.hasGpu" class="hw-card">
      <div class="hw-ico"><SettingsIcon name="cpu" :size="22" /></div>
      <div class="hw-info">
        <div class="hw-name">{{ info?.gpuName || '已偵測到 GPU' }}</div>
        <div class="hw-meta">後端 {{ backendLabel }}</div>
      </div>
      <span class="badge dn">✓ 已啟用</span>
    </div>
    <div v-else class="hw-card">
      <div class="hw-info"><div class="hw-name">未偵測到 GPU → 使用 CPU</div></div>
    </div>

    <div class="hw-note">
      即時字幕會用到這個加速。<b>未偵測到 GPU 時仍可用 CPU 執行</b>，但速度有限、建議改用 small / turbo 模型；large-v3 在純 CPU 下會停用。
    </div>
  </div>
</template>

<style scoped>
.p-title { font-size: 15px; font-weight: 600; color: #fff; margin: 0 0 3px; }
.p-sub { font-size: 12px; color: #8a8a92; margin: 0 0 14px; }
.field { display: flex; align-items: center; justify-content: space-between; padding: 11px 2px; }
.lab { color: #d8d8de; }
.lab small { display: block; color: #7a7a82; font-size: 11px; margin-top: 1px; }
.hw-card { display: flex; align-items: center; gap: 14px; padding: 14px; background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.09); border-radius: 10px; margin-top: 4px; }
.hw-ico { width: 42px; height: 42px; border-radius: 9px; background: rgba(var(--accent-rgb),0.16); color: var(--accent); display: flex; align-items: center; justify-content: center; flex: none; }
.hw-info { flex: 1; min-width: 0; }
.hw-name { font-weight: 600; color: #fff; }
.hw-meta { font-size: 12px; color: #8a8a92; margin-top: 2px; }
.badge { font-size: 11px; padding: 4px 12px; border-radius: 999px; }
.badge.dn { background: #143524; color: #4ade80; border: 1px solid #1f5036; }
.hw-note { margin-top: 14px; font-size: 12px; color: #8f8f97; line-height: 1.65; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); border-left: 3px solid var(--accent); border-radius: 8px; padding: 11px 13px; }
.hw-note b { color: #c8c8cf; font-weight: 600; }
</style>
