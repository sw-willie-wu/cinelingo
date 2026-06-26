<script setup lang="ts">
/**
 * SidePanel — 靠右浮動玻璃側欄的共用 chrome。
 * 上緣貼 titlebar 底、下緣停控制鈕上方；Teleport 到 body（不隨自動隱藏的控制列消失）；
 * scrim 點外側關閉；由右滑入；內建細灰玻璃滾動條 + scrollbar-gutter。
 * 內容用 slot：#header（固定頂、自帶分隔線由消費端決定）/ 預設（可捲動主體）/ #footer（固定底）。
 */
defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <Teleport to="body">
    <Transition name="sp">
      <div v-if="open" class="sp-scrim" @click.self="emit('close')">
        <aside class="sp-card" @click.stop>
          <slot name="header" />
          <div class="sp-body"><slot /></div>
          <slot name="footer" />
        </aside>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.sp-scrim { position: fixed; inset: 0; z-index: 45; }
.sp-card {
  position: absolute; top: 40px; right: 14px; bottom: 60px; width: 360px; max-width: 86vw;
  display: flex; flex-direction: column;
  background: rgba(24,25,30,0.92); backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4);
  border: 1px solid rgba(255,255,255,0.12); border-radius: 14px; box-shadow: 0 20px 55px rgba(0,0,0,.55);
  color: #e8e8ea; font-size: 13px; overflow: hidden;
}
.sp-body { flex: 1; overflow-y: auto; scrollbar-gutter: stable; }
.sp-body::-webkit-scrollbar { width: 10px; }
.sp-body::-webkit-scrollbar-track { background: transparent; }
.sp-body::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.16); border-radius: 6px; border: 2px solid transparent; background-clip: padding-box; }
.sp-body::-webkit-scrollbar-thumb:hover { background: rgba(255,255,255,0.3); background-clip: padding-box; }

/* 滑入：scrim 淡入 + card 由右滑入 */
.sp-enter-active, .sp-leave-active { transition: opacity .22s ease; }
.sp-enter-active .sp-card, .sp-leave-active .sp-card { transition: transform .26s cubic-bezier(.4,0,.2,1); }
.sp-enter-from, .sp-leave-to { opacity: 0; }
.sp-enter-from .sp-card, .sp-leave-to .sp-card { transform: translateX(110%); }
</style>
