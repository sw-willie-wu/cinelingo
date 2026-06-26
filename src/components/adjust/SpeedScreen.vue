<script setup lang="ts">
import { useSpeed } from '../../player/useSpeed'
defineEmits<{ back: [] }>()
const { speed, presets, setSpeed } = useSpeed()
</script>

<template>
  <div class="back" @click="$emit('back')"><span class="bk">‹</span><span class="ttl">播放速度</span></div>
  <div class="content">
    <div class="chips">
      <button
        v-for="p in presets"
        :key="p"
        class="chip"
        :class="{ active: speed === p }"
        @click="setSpeed(p)"
      >{{ p }}×</button>
    </div>
    <div class="fine">
      <span class="fine-lbl">微調</span>
      <div class="stepper">
        <button @click="setSpeed(speed - 0.05)">−</button>
        <span class="val">{{ speed }}×</span>
        <button @click="setSpeed(speed + 0.05)">＋</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.back { display:flex; align-items:center; gap:10px; padding:12px 14px; border-bottom:1px solid rgba(255,255,255,0.08); cursor:pointer; }
.bk { font-size:16px; color:#cfd2db; }
.ttl { color:#fff; font-weight:600; }

.content { padding: 14px 12px; display: flex; flex-direction: column; gap: 16px; }

.chips { display: flex; flex-wrap: wrap; gap: 8px; }
.chip {
  padding: 6px 14px;
  border-radius: 20px;
  border: 1px solid rgba(255,255,255,0.12);
  background: rgba(255,255,255,0.06);
  color: #cfd2db;
  font-size: 13px;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}
.chip:hover { background: rgba(255,255,255,0.12); }
.chip.active {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}

.fine { display: flex; align-items: center; justify-content: space-between; padding-top: 4px; border-top: 1px solid rgba(255,255,255,0.08); }
.fine-lbl { color: #cfd2db; font-size: 13px; }
.stepper { display: flex; align-items: center; gap: 2px; background: rgba(255,255,255,0.06); border-radius: 8px; padding: 2px; }
.stepper button { width: 26px; height: 24px; border: none; background: none; color: #cfd2db; font-size: 15px; cursor: pointer; border-radius: 6px; }
.stepper button:hover { background: rgba(255,255,255,0.1); }
.val { min-width: 52px; text-align: center; color: #fff; font-variant-numeric: tabular-nums; font-size: 13px; }
</style>
