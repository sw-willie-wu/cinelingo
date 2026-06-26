<script setup lang="ts">
// 置中載入指示：SVG 細弧（淡底環 + accent 弧）旋轉 + 中央可選 %/文字。緩衝給 percent、resolve 兩者皆不給＝純弧。
// 用 SVG stroke（WebView2 穩定支援）而非 conic-gradient+mask（部分 WebView2 不渲染）。
defineProps<{ percent?: number | null; label?: string }>()
</script>

<template>
  <div class="loading-overlay" role="status" aria-label="載入中">
    <svg class="ring" viewBox="0 0 40 40" aria-hidden="true">
      <circle class="bg" cx="20" cy="20" r="17" />
      <circle class="arc" cx="20" cy="20" r="17" />
    </svg>
    <span v-if="percent != null" class="cap">{{ Math.round(percent) }}%</span>
    <span v-else-if="label" class="cap">{{ label }}</span>
  </div>
</template>

<style scoped>
.loading-overlay {
  position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); z-index: 8;
  width: 72px; height: 72px; pointer-events: none;
  display: flex; align-items: center; justify-content: center;
  background: rgba(20, 20, 24, 0.72); border-radius: 50%; backdrop-filter: blur(8px);
}
.ring { width: 56px; height: 56px; animation: lspin 0.9s linear infinite; }
.ring circle { fill: none; stroke-width: 3; }
.bg { stroke: rgba(255, 255, 255, 0.16); }
.arc { stroke: var(--accent); stroke-linecap: round; stroke-dasharray: 30 200; }
.cap {
  position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; z-index: 1;
  font-size: 13px; line-height: 1; font-variant-numeric: tabular-nums; color: #e8e8ea;
}
@keyframes lspin { to { transform: rotate(360deg); } }
</style>
