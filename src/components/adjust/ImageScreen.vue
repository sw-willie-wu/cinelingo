<script setup lang="ts">
import { useVideoAdjust } from '../../player/useVideoAdjust'
import GlassToggle from '../GlassToggle.vue'
import type { ImageProp } from '../../mpv'

defineEmits<{ back: [] }>()

const { state, props, setProp, setDebandOn, reset } = useVideoAdjust()

const LABELS: Record<ImageProp, string> = {
  brightness: '亮度',
  contrast: '對比',
  saturation: '飽和',
  gamma: 'Gamma',
  hue: '色相',
}
</script>

<template>
  <div class="is-wrap">
    <div class="back">
      <span class="bk" @click="$emit('back')">‹</span>
      <span class="ttl">影像調整</span>
      <span class="reset" @click="reset()">重設</span>
    </div>

    <div class="rows">
      <div v-for="p in props" :key="p" class="row">
        <span class="lbl">{{ LABELS[p] }}</span>
        <input
          type="range"
          min="-100"
          max="100"
          step="1"
          :value="state[p]"
          class="slider"
          @input="setProp(p, Number(($event.target as HTMLInputElement).value))"
        />
        <span class="val">{{ state[p] > 0 ? '+' + state[p] : state[p] }}</span>
      </div>

      <div class="row deband-row">
        <span class="lbl">去色帶（deband）</span>
        <GlassToggle
          :model-value="state.deband"
          @update:model-value="setDebandOn($event)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.is-wrap {
  display: flex;
  flex-direction: column;
  color: #e8e8ea;
  font-size: 13px;
}

/* ── header ── */
.back {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}
.bk {
  font-size: 18px;
  color: #cfd2db;
  cursor: pointer;
  line-height: 1;
  padding: 0 2px;
}
.bk:hover { color: #fff; }
.ttl {
  flex: 1;
  color: #fff;
  font-weight: 600;
}
.reset {
  font-size: 12px;
  color: #7eb8ff;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 5px;
}
.reset:hover { background: rgba(255, 255, 255, 0.08); }

/* ── slider rows ── */
.rows {
  display: flex;
  flex-direction: column;
  padding: 6px 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 14px;
}
.row:not(:last-child) {
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}
.lbl {
  width: 72px;
  flex: none;
  color: #cfd2db;
  font-size: 13px;
}
.slider {
  flex: 1;
  -webkit-appearance: none;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: rgba(255, 255, 255, 0.15);
  outline: none;
  cursor: pointer;
}
.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent, #3a8cf0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  cursor: pointer;
}
.slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent, #3a8cf0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  border: none;
  cursor: pointer;
}
.val {
  width: 36px;
  text-align: right;
  flex: none;
  color: #fff;
  font-variant-numeric: tabular-nums;
  font-size: 12px;
}

/* ── deband row ── */
.deband-row {
  margin-top: 4px;
  border-top: 1px solid rgba(255, 255, 255, 0.08) !important;
  border-bottom: none !important;
}
.deband-row .lbl {
  flex: 1;
  width: auto;
  color: #cfd2db;
}
</style>
