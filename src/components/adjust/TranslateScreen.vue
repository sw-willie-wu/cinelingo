<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { useSettings } from '../../player/useSettings'
import { LANGS } from '../../player/langs'
import { translateEngineReady, provisionTranslateEngine } from '../../player/backend'
import GlassSelect from '../GlassSelect.vue'
const emit = defineEmits<{ back: [] }>()
const settings = useSettings()
const ls = computed(() => settings.state.liveSubs)
// 翻譯目標排除 auto（要明確語言）
const langOpts = LANGS.filter((l) => l.value !== 'auto').map((l) => ({ value: l.value, label: l.label }))
const ready = ref(false)
const downloading = ref(false)
onMounted(async () => { ready.value = await translateEngineReady().catch(() => false) })
async function getEngine() {
  downloading.value = true
  try { await provisionTranslateEngine(); ready.value = await translateEngineReady() }
  finally { downloading.value = false }
}
</script>
<template>
  <div class="back" @click="emit('back')"><span class="bk">‹</span><span class="ttl">即時翻譯</span></div>

  <div class="row">
    <span>啟用翻譯</span>
    <input type="checkbox" :checked="ls.translateEnabled" :disabled="!ready"
      @change="ls.translateEnabled = ($event.target as HTMLInputElement).checked" />
  </div>
  <div class="row">
    <span>翻譯成</span>
    <GlassSelect :model-value="ls.translateTo" :options="langOpts" :disabled="!ready"
      @update:model-value="ls.translateTo = $event" />
  </div>

  <div v-if="!ready" class="provision">
    <p class="hint">即時翻譯需下載本地模型（約 2.9GB，含 llama 引擎 + gemma-3-4b）。低於 8GB 顯卡可能不順。</p>
    <button :disabled="downloading" @click="getEngine">{{ downloading ? '下載中…' : '下載翻譯模型' }}</button>
  </div>
  <p v-else class="hint ok">✓ 翻譯引擎已備妥（本地、離線）。</p>
</template>
<style scoped>
.back { display:flex; align-items:center; gap:10px; padding:12px 14px; border-bottom:1px solid rgba(255,255,255,0.08); cursor:pointer; }
.bk { font-size:16px; color:#cfd2db; } .ttl { color:#fff; font-weight:600; }
.row { display:flex; align-items:center; justify-content:space-between; padding:11px 14px; }
.row > span { color:#cfd2db; }
.provision { padding:10px 14px; } .hint { font-size:12px; color:#8a8d99; margin:0 0 8px; } .hint.ok { color:#7fd18a; }
.provision button { width:100%; padding:9px; background:rgba(var(--accent-rgb),0.16); border:1px solid var(--accent); border-radius:9px; color:#dfe7ff; cursor:pointer; }
.provision button:disabled { opacity:.5; cursor:default; }
</style>
