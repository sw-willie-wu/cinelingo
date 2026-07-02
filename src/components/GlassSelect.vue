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
  const gap = 4, margin = 8 // 選單與按鈕間距 / 離視窗邊緣留白
  const spaceBelow = window.innerHeight - r.bottom - gap - margin
  const spaceAbove = r.top - gap - margin
  // 下方夠(≥220)就往下；否則若上方較寬就往上長。兩向都限高 + 滾動，避免被視窗邊緣截掉。
  const down = spaceBelow >= 220 || spaceBelow >= spaceAbove
  // 收短：最多 ~8 列高，且不超過該方向可用空間（清單再長就內部捲動）。
  const maxH = Math.max(140, Math.min(288, Math.floor(down ? spaceBelow : spaceAbove)))
  // 寬度：至少和按鈕一樣寬，可長到內容寬（避免長選項被截 → 橫向捲軸）；上限不超出視窗右緣。
  const style: Record<string, string> = {
    left: `${r.left}px`,
    minWidth: `${r.width}px`,
    maxWidth: `${Math.max(r.width, window.innerWidth - r.left - margin)}px`,
    maxHeight: `${maxH}px`,
  }
  if (down) style.top = `${r.bottom + gap}px`
  else style.bottom = `${window.innerHeight - r.top + gap}px`
  menuStyle.value = style
}
function onScroll(e: Event) {
  // 選單內部捲動不關閉；只有底層頁面捲動(按鈕會位移)才關。
  if (menuEl.value && e.target instanceof Node && menuEl.value.contains(e.target)) return
  close()
}
function close() {
  if (!open.value) return
  open.value = false
  document.removeEventListener('click', onDocClick, true)
  window.removeEventListener('scroll', onScroll, true)
  window.removeEventListener('resize', close)
}
function toggle() {
  if (props.disabled) return
  if (open.value) { close(); return }
  open.value = true
  place()
  document.addEventListener('click', onDocClick, true)
  window.addEventListener('scroll', onScroll, true)
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
      <span :lang="modelValue">{{ current }}</span>
      <svg class="gsel-chev" :class="{ up: open }" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6" /></svg>
    </button>
    <Teleport to="body">
      <div v-if="open" ref="menuEl" class="gsel-menu" :style="menuStyle">
        <button v-for="o in options" :key="o.value" type="button" class="gsel-opt" :class="{ on: o.value === modelValue }" :lang="o.value" @click="select(o.value)">{{ o.label }}</button>
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
  overflow-y: auto; overflow-x: hidden; overscroll-behavior: contain;
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
