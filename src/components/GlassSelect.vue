<script setup lang="ts">
/**
 * 通用玻璃風下拉選單（取代原生 <select>，連展開清單都可樣式化）。
 * 用法：<GlassSelect v-model="x" :options="[{value,label},...]" />
 * 選單以 Teleport 掛到 body + fixed 定位，避免被 modal 的 overflow 裁切。
 */
import { ref, computed, onBeforeUnmount } from 'vue'

const props = defineProps<{ modelValue: string; options: { value: string; label: string }[]; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [string] }>()

const open = ref(false)
const btn = ref<HTMLElement | null>(null)
const menuEl = ref<HTMLElement | null>(null)
const menuStyle = ref<Record<string, string>>({})
const current = computed(() => props.options.find((o) => o.value === props.modelValue)?.label ?? '')

function place() {
  const r = btn.value?.getBoundingClientRect()
  if (!r) return
  menuStyle.value = { top: `${r.bottom + 4}px`, left: `${r.left}px`, width: `${r.width}px` }
}
function close() {
  if (!open.value) return
  open.value = false
  document.removeEventListener('click', onDocClick, true)
  window.removeEventListener('scroll', close, true)
  window.removeEventListener('resize', close)
}
function toggle() {
  if (props.disabled) return
  if (open.value) { close(); return }
  open.value = true
  place()
  document.addEventListener('click', onDocClick, true)
  window.addEventListener('scroll', close, true)
  window.addEventListener('resize', close)
}
function onDocClick(e: MouseEvent) {
  const t = e.target as Node
  if (btn.value?.contains(t) || menuEl.value?.contains(t)) return
  close()
}
function select(v: string) { emit('update:modelValue', v); close() }
onBeforeUnmount(close)
</script>

<template>
  <div class="gsel">
    <button ref="btn" type="button" class="gsel-btn" :disabled="disabled" @click.stop="toggle">
      <span>{{ current }}</span>
      <svg class="gsel-chev" :class="{ up: open }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6" /></svg>
    </button>
    <Teleport to="body">
      <div v-if="open" ref="menuEl" class="gsel-menu" :style="menuStyle">
        <button v-for="o in options" :key="o.value" type="button" class="gsel-opt" :class="{ on: o.value === modelValue }" @click="select(o.value)">{{ o.label }}</button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.gsel { position: relative; }
.gsel-btn {
  display: flex; align-items: center; justify-content: space-between; gap: 8px; min-width: 96px;
  background: rgba(255,255,255,0.06); color: #fff; border: 1px solid rgba(255,255,255,0.14);
  border-radius: 7px; padding: 6px 10px; font-size: 12px; cursor: pointer;
}
.gsel-btn:hover { background: rgba(255,255,255,0.1); }
.gsel-btn:disabled { opacity: .5; cursor: not-allowed; }
.gsel-chev { width: 12px; height: 12px; color: #9aa0a8; transition: transform .15s; flex: none; }
.gsel-chev.up { transform: rotate(180deg); }
.gsel-menu {
  position: fixed; z-index: 70; box-sizing: border-box; padding: 4px; display: flex; flex-direction: column; gap: 1px;
  background: rgba(26,26,30,0.72); backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4);
  border: 1px solid rgba(255,255,255,0.12); border-radius: 9px; box-shadow: 0 14px 34px rgba(0,0,0,0.55);
}
.gsel-opt {
  text-align: left; white-space: nowrap; background: none; border: none; color: #d8d8de;
  font-size: 12px; padding: 7px 10px 7px 6px; border-radius: 6px; cursor: pointer;
}
.gsel-opt:hover { background: rgba(255,255,255,0.08); }
.gsel-opt.on { background: var(--accent); color: #fff; }
</style>
