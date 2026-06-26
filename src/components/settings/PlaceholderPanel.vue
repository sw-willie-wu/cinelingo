<script setup lang="ts">
import SettingsIcon from './SettingsIcon.vue'
defineProps<{ title: string; icon: string }>()
</script>
<template>
  <div class="ph">
    <div class="ph-ico"><SettingsIcon :name="icon" :size="26" /></div>
    <div class="ph-title">{{ title }}設定</div>
    <div class="ph-text">此區規劃中，之後版本會加入相關設定。</div>
    <div class="ph-badge">規劃中</div>
  </div>
</template>
<style scoped>
.ph { height: 100%; display: flex; flex-direction: column; align-items: center; justify-content: center; text-align: center; color: #6b6b73; }
.ph-ico { width: 54px; height: 54px; margin-bottom: 14px; border-radius: 14px; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.08); display: flex; align-items: center; justify-content: center; color: #9a9aa2; }
.ph-title { font-size: 15px; font-weight: 600; color: #9a9aa2; margin-bottom: 5px; }
.ph-text { font-size: 12px; }
.ph-badge { margin-top: 12px; font-size: 11px; color: #7a7a82; border: 1px solid rgba(255,255,255,0.12); border-radius: 999px; padding: 3px 12px; }
</style>
