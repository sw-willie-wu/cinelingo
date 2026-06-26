<script setup lang="ts">
/**
 * 通用開關（玻璃風，含滑動動畫）。
 * 用法：<GlassToggle v-model="on" />　或唯讀：<GlassToggle :model-value="x" disabled />
 */
const props = defineProps<{ modelValue: boolean; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [boolean] }>()
function toggle() {
  if (!props.disabled) emit('update:modelValue', !props.modelValue)
}
</script>

<template>
  <button
    type="button"
    class="gtog"
    :class="{ off: !modelValue }"
    :disabled="disabled"
    :aria-pressed="modelValue"
    @mousedown.prevent
    @click.stop="toggle"
  ></button>
</template>

<style scoped>
.gtog {
  width: 30px; height: 16px; border-radius: 8px; position: relative; flex: none; padding: 0; border: none;
  background: var(--accent); cursor: pointer; box-shadow: inset 0 0 0 1px rgba(255,255,255,0.09);
  transition: background .16s ease;
}
.gtog::after {
  content: ''; position: absolute; top: 2px; left: 2px; width: 12px; height: 12px; border-radius: 50%;
  background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,0.4); transform: translateX(14px); transition: transform .16s ease;
}
.gtog.off { background: rgba(255,255,255,0.18); }
.gtog.off::after { transform: translateX(0); }
.gtog:disabled { opacity: .6; cursor: default; }
</style>
