<script setup lang="ts">
import { useAudioAdjust } from '../../player/useAudioAdjust'
import GlassToggle from '../GlassToggle.vue'
defineEmits<{ back: [] }>()
const { normalize, setNormalize } = useAudioAdjust()
</script>

<template>
  <div class="ns-wrap">
    <div class="back" @click="$emit('back')">
      <span class="bk">‹</span>
      <span class="ttl">音量正規化</span>
    </div>
    <div class="rows">
      <div class="row">
        <span class="lbl">音量正規化</span>
        <GlassToggle
          :model-value="normalize()"
          @update:model-value="setNormalize($event)"
        />
      </div>
      <div class="hint">拉平忽大忽小的音量（dynaudnorm），看夜場電影/對白很實用</div>
    </div>
  </div>
</template>

<style scoped>
.ns-wrap {
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
  justify-content: space-between;
  gap: 10px;
  padding: 11px 14px;
}
.lbl {
  color: #cfd2db;
}
.hint {
  padding: 4px 14px 12px;
  font-size: 11px;
  color: #7f8290;
  line-height: 1.5;
}
</style>
