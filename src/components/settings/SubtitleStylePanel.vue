<script setup lang="ts">
import { ref, computed } from 'vue'
import { useSettings } from '../../player/useSettings'
import { subTextStyle, type OutlineLevel } from '../../player/settings'

const settings = useSettings()
const target = ref<'primary' | 'secondary'>('primary')
const style = computed(() => settings.state.appearance[target.value])

const COLORS = ['#ffffff', '#ffe14d', '#9cf06b', '#6bc7ff']
const isCustom = computed(() => !COLORS.includes(style.value.color))
const OUTLINES: { v: OutlineLevel; l: string }[] = [
  { v: 'none', l: '無' }, { v: 'thin', l: '細' }, { v: 'mid', l: '中' }, { v: 'thick', l: '粗' },
]
// 預覽用固定容器高（面板內），實際播放縮放在 overlay 處理
const PREVIEW_H = 720
const primaryCss = computed(() => subTextStyle(settings.state.appearance.primary, PREVIEW_H))
const secondaryCss = computed(() => subTextStyle(settings.state.appearance.secondary, PREVIEW_H))
</script>

<template>
  <div>
    <div class="p-title">字幕外觀</div>
    <div class="p-sub">主字幕為主要字幕（如譯文）；第二字幕可同時顯示另一語言（如原文）。</div>

    <div class="preview">
      <div class="caps">
        <div :style="secondaryCss">Subtitle preview · 原文</div>
        <div :style="primaryCss">這是主字幕預覽 · 譯文</div>
      </div>
    </div>
    <div class="hint">實際播放時第二字幕套用於翻譯功能（即將推出）；此處為樣式預覽。</div>

    <div class="field">
      <div class="lab">一行最大寬度（套用所有字幕）</div>
      <input type="range" min="50" max="90" step="5" v-model.number="settings.state.appearance.maxWidthPct" />
      <span class="val">{{ settings.state.appearance.maxWidthPct }}%</span>
    </div>

    <div class="editsel">
      <button :class="{ on: target === 'primary' }" @click="target = 'primary'">主字幕</button>
      <button :class="{ on: target === 'secondary' }" @click="target = 'secondary'">第二字幕</button>
    </div>

    <div class="field">
      <div class="lab">字級</div>
      <input type="range" min="14" max="48" v-model.number="style.fontSize" />
      <span class="val">{{ style.fontSize }} px</span>
    </div>
    <div class="field">
      <div class="lab">垂直位置（距底部）</div>
      <input type="range" min="0" max="40" v-model.number="style.bottomPct" />
      <span class="val">{{ style.bottomPct }}%</span>
    </div>
    <div class="field">
      <div class="lab">文字顏色</div>
      <div class="swatches">
        <button v-for="c in COLORS" :key="c" class="sw-c" :class="{ on: style.color === c }" :style="{ background: c }" @click="style.color = c"></button>
        <span class="sw-div"></span>
        <label class="sw-c custom" :class="{ on: isCustom }" :style="isCustom ? { background: style.color } : undefined" aria-label="自訂顏色">
          <input type="color" :value="style.color" @input="style.color = ($event.target as HTMLInputElement).value" />
        </label>
      </div>
    </div>
    <div class="field">
      <div class="lab">外框</div>
      <div class="seg">
        <button v-for="o in OUTLINES" :key="o.v" :class="{ on: style.outline === o.v }" @click="style.outline = o.v">{{ o.l }}</button>
      </div>
    </div>
    <div class="field">
      <div class="lab">背景底</div>
      <div class="seg">
        <button :class="{ on: style.background === 'none' }" @click="style.background = 'none'">無</button>
        <button :class="{ on: style.background === 'translucent' }" @click="style.background = 'translucent'">半透明黑底</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.p-title { font-size: 15px; font-weight: 600; color: #fff; margin: 0 0 3px; }
.p-sub { font-size: 12px; color: #8a8a92; margin: 0 0 12px; }
.preview { height: 138px; border-radius: 10px; background: linear-gradient(160deg,#243246,#0d1117); display: flex; align-items: flex-end; justify-content: center; padding-bottom: 14px; border: 1px solid rgba(255,255,255,0.09); }
.caps { display: flex; flex-direction: column; align-items: center; gap: 5px; }
.hint { font-size: 11px; color: #6b6b73; margin: 6px 0 6px; }
.field { display: flex; align-items: center; gap: 12px; justify-content: space-between; padding: 9px 2px; }
.lab { color: #d8d8de; flex: 1; }
.val { width: 50px; text-align: right; font-size: 12px; color: #cdd3da; }
input[type=range] {
  width: 200px; height: 5px; -webkit-appearance: none; appearance: none;
  background: rgba(255,255,255,0.16); border-radius: 3px; cursor: pointer; outline: none;
}
input[type=range]::-webkit-slider-thumb {
  -webkit-appearance: none; appearance: none;
  width: 15px; height: 15px; border-radius: 50%; background: var(--accent);
  box-shadow: 0 1px 3px rgba(0,0,0,0.5); cursor: pointer;
}
.editsel { display: flex; background: rgba(0,0,0,0.22); border: 1px solid rgba(255,255,255,0.07); border-radius: 9px; padding: 3px; margin: 4px 0 2px; }
.editsel button { flex: 1; padding: 7px; border: none; background: none; border-radius: 6px; color: #bdbdc4; cursor: pointer; font-size: 13px; }
.editsel button.on { background: var(--accent); color: #fff; font-weight: 600; }
.swatches { display: flex; gap: 11px; align-items: center; }
.sw-div { width: 1px; height: 18px; background: rgba(255,255,255,0.18); flex: none; }
.sw-c { width: 20px; height: 20px; border-radius: 50%; border: none; cursor: pointer; box-shadow: 0 0 0 1px #00000066; padding: 0; }
.sw-c.on { box-shadow: 0 0 0 2px #202024, 0 0 0 4px var(--accent); transform: scale(1.1); }
.sw-c.custom { background: conic-gradient(from 0deg, #ff5b5b, #ffe14d, #9cf06b, #6bc7ff, #b07bff, #ff5b5b); position: relative; overflow: hidden; padding: 0; }
.sw-c.custom input { position: absolute; inset: 0; width: 100%; height: 100%; opacity: 0; border: none; padding: 0; cursor: pointer; }
.seg { display: flex; border: 1px solid rgba(255,255,255,0.14); border-radius: 7px; overflow: hidden; }
.seg button { padding: 5px 14px; font-size: 12px; color: #bdbdc4; cursor: pointer; background: none; border: none; }
.seg button.on { background: var(--accent); color: #fff; }
</style>
