<script setup lang="ts">
import { useAudioAdjust } from '../../player/useAudioAdjust'
defineEmits<{ back: [] }>()
const { audioDelayMs, setAudioDelay } = useAudioAdjust()
</script>

<template>
  <div class="ads-wrap">
    <div class="back" @click="$emit('back')">
      <span class="bk">‹</span>
      <span class="ttl">音訊延遲（A/V 同步）</span>
    </div>
    <div class="rows">
      <div class="row">
        <input
          type="range"
          min="-2000"
          max="2000"
          step="50"
          class="slider"
          :value="audioDelayMs"
          @input="setAudioDelay(Number(($event.target as HTMLInputElement).value))"
        />
        <span class="val">{{ audioDelayMs > 0 ? '+' + audioDelayMs : audioDelayMs }}ms</span>
      </div>
      <div class="hint">影音不同步時微調；正值＝音訊延後</div>
    </div>
  </div>
</template>

<style scoped>
.ads-wrap {
  display: flex;
  flex-direction: column;
  color: #e8e8ea;
  font-size: 13px;
}

.back {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  cursor: pointer;
}
.bk {
  font-size: 18px;
  color: #cfd2db;
  line-height: 1;
  padding: 0 2px;
}
.bk:hover { color: #fff; }
.ttl {
  flex: 1;
  color: #fff;
  font-weight: 600;
}

.rows {
  display: flex;
  flex-direction: column;
  padding: 6px 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 11px 14px;
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
  min-width: 54px;
  text-align: right;
  flex: none;
  color: #fff;
  font-variant-numeric: tabular-nums;
  font-size: 12px;
}
.hint {
  padding: 4px 14px 12px;
  font-size: 11px;
  color: #7f8290;
  line-height: 1.5;
}
</style>
