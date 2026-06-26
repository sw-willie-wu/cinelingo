<script setup lang="ts">
import { useAudioAdjust } from '../../player/useAudioAdjust'
import { EQ_FREQS, EQ_PRESETS } from '../../player/audioAf'
import GlassToggle from '../GlassToggle.vue'

defineEmits<{ back: [] }>()

const { eq, setEqEnabled, setBand, setPreset } = useAudioAdjust()

const PRESET_LABELS: Record<string, string> = {
  flat: '平坦', bass: '低音', vocal: '人聲', classic: '古典', rock: '搖滾', jazz: '爵士',
}
const presetKeys = Object.keys(EQ_PRESETS)

function freqLabel(f: number): string { return f >= 1000 ? `${f / 1000}k` : `${f}` }
</script>

<template>
  <div class="eq-wrap">
    <!-- Header -->
    <div class="back" @click="$emit('back')">
      <span class="bk">‹</span>
      <span class="ttl">等化器</span>
    </div>

    <!-- Master toggle -->
    <div class="toggle-row">
      <span class="toggle-lbl">啟用</span>
      <GlassToggle
        :model-value="eq.enabled"
        @update:model-value="setEqEnabled($event)"
      />
    </div>

    <!-- Preset chips -->
    <div class="presets">
      <button
        v-for="key in presetKeys"
        :key="key"
        class="chip"
        :class="{ active: eq.preset === key }"
        @click="setPreset(key)"
      >{{ PRESET_LABELS[key] ?? key }}</button>
      <span class="chip custom-chip" :class="{ active: eq.preset === 'custom' }">自訂</span>
    </div>

    <!-- 10-band vertical sliders -->
    <div class="bands" :class="{ disabled: !eq.enabled }">
      <div v-for="(freq, i) in EQ_FREQS" :key="freq" class="band">
        <span class="gain">{{ eq.bands[i] > 0 ? '+' + eq.bands[i] : eq.bands[i] }}</span>
        <input
          type="range"
          class="vslider"
          min="-12"
          max="12"
          step="1"
          :value="eq.bands[i]"
          :disabled="!eq.enabled"
          @input="setBand(i, Number(($event.target as HTMLInputElement).value))"
        />
        <span class="freq">{{ freqLabel(freq) }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.eq-wrap {
  display: flex;
  flex-direction: column;
  color: #e8e8ea;
  font-size: 13px;
}

/* header */
.back {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  cursor: pointer;
}
.bk { font-size: 16px; color: #cfd2db; }
.bk:hover { color: #fff; }
.ttl { color: #fff; font-weight: 600; }

/* master toggle row */
.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 11px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}
.toggle-lbl { color: #cfd2db; }

/* preset chips */
.presets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 10px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}
.chip {
  padding: 4px 10px;
  border-radius: 20px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(255, 255, 255, 0.06);
  color: #cfd2db;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  line-height: 1.4;
}
.chip:hover { background: rgba(255, 255, 255, 0.12); }
.chip.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}
.custom-chip {
  cursor: default;
}
.custom-chip:hover { background: rgba(255, 255, 255, 0.06); }
.custom-chip.active:hover { background: var(--accent); }

/* 10-band EQ grid */
.bands {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding: 12px 10px 10px;
  gap: 2px;
}
.bands.disabled { opacity: 0.45; pointer-events: none; }

.band {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  flex: 1;
  min-width: 0;
}

.gain {
  font-size: 9px;
  color: #8a8d99;
  font-variant-numeric: tabular-nums;
  height: 12px;
  line-height: 12px;
  white-space: nowrap;
}

/* vertical range slider */
.vslider {
  -webkit-appearance: slider-vertical;
  appearance: slider-vertical;
  writing-mode: vertical-lr;
  direction: rtl;
  width: 20px;
  height: 100px;
  cursor: pointer;
  background: transparent;
  outline: none;
  padding: 0;
}

/* WebKit track */
.vslider::-webkit-slider-runnable-track {
  width: 4px;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}
.vslider::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent, #3a8cf0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  margin-left: -5px;
  cursor: pointer;
}

/* Firefox track */
.vslider::-moz-range-track {
  width: 4px;
  background: rgba(255, 255, 255, 0.15);
  border-radius: 2px;
}
.vslider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--accent, #3a8cf0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  border: none;
  cursor: pointer;
}

.freq {
  font-size: 9px;
  color: #6b6e7a;
  text-align: center;
  white-space: nowrap;
}
</style>
