<script setup lang="ts">
import { computed } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useSubtitles } from '../../player/useSubtitles'
import { useSettings } from '../../player/useSettings'
import { readTextFile } from '../../player/backend'
import { parseSubtitle } from '../../player/subtitles'
import { LANGS } from '../../player/langs'
import GlassSelect from '../GlassSelect.vue'

const props = defineProps<{ track: 'primary' | 'secondary' }>()
const emit = defineEmits<{ back: [] }>()

const subs = useSubtitles()
const settings = useSettings()

const masterOn = computed(() => settings.state.liveSubs.enabled)
const cur = computed(() => subs.tracks[props.track])
const langOpts = LANGS.map((l) => ({ value: l.value, label: l.label }))

const delayText = computed(() => {
  const v = cur.value.delaySec
  return `${v > 0 ? '+' : ''}${v.toFixed(1)}s`
})

function bump(d: number) {
  subs.setDelay(props.track, Math.round((cur.value.delaySec + d) * 10) / 10)
}

async function addFile() {
  const sel = await openDialog({ filters: [{ name: '字幕', extensions: ['srt', 'vtt'] }] })
  if (typeof sel !== 'string') return
  let text: string
  try { text = await readTextFile(sel) } catch { return }
  const name = sel.replace(/^.*[\\/]/, '')
  const cues = parseSubtitle(name, text)
  const id = subs.addFile(name, cues, sel, true)
  subs.selectSource(props.track, id)
}
</script>

<template>
  <div class="back" @click="emit('back')">
    <span class="bk">‹</span>
    <span class="ttl">{{ props.track === 'primary' ? '主字幕' : '第二字幕' }}</span>
  </div>

  <div class="lbl">來源</div>
  <ul class="srcs">
    <li
      class="src"
      :class="{ sel: cur.source === 'live', disabled: !masterOn }"
      @click="masterOn && subs.selectSource(props.track, 'live')"
    >
      <span>即時字幕（Whisper）</span>
      <span v-if="!masterOn" class="hint">需在設定開啟</span>
    </li>
    <li
      v-for="f in subs.files.value"
      :key="f.id"
      class="src"
      :class="{ sel: cur.source === f.id }"
      @click="subs.selectSource(props.track, f.id)"
    >
      <span class="fname">{{ f.name }}</span>
    </li>
    <li
      class="src"
      :class="{ sel: cur.source === 'off' }"
      @click="subs.selectSource(props.track, 'off')"
    >
      <span>關閉字幕</span>
    </li>
  </ul>

  <div class="langrow">
    <span>即時字幕語言</span>
    <GlassSelect
      :model-value="settings.state.liveSubs.sourceLang"
      :options="langOpts"
      :disabled="!masterOn"
      @update:model-value="settings.state.liveSubs.sourceLang = $event"
    />
  </div>

  <button class="add" @click="addFile">＋ 新增字幕檔…</button>

  <div class="delay">
    <span>此軌延遲</span>
    <div class="stepper">
      <button @click="bump(-0.1)">−</button>
      <span class="val">{{ delayText }}</span>
      <button @click="bump(0.1)">＋</button>
    </div>
  </div>
</template>

<style scoped>
.back { display: flex; align-items: center; gap: 10px; padding: 12px 14px; border-bottom: 1px solid rgba(255,255,255,0.08); cursor: pointer; }
.bk { font-size: 16px; color: #cfd2db; }
.ttl { color: #fff; font-weight: 600; }

.lbl { font-size: 10px; letter-spacing: .5px; text-transform: uppercase; color: #7f8290; padding: 11px 14px 5px; }

.srcs { list-style: none; margin: 0; padding: 0 6px; }
.src { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 9px; cursor: pointer; border: 1px solid transparent; }
.src:hover { background: rgba(255,255,255,0.06); }
.src.sel { background: rgba(var(--accent-rgb),0.16); border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent) inset; }
.src.disabled { opacity: .42; cursor: not-allowed; }
.src.disabled:hover { background: none; }
.hint { margin-left: auto; font-size: 10px; color: #8a8d99; }
.fname { color: #dfe2ea; word-break: break-all; }

.langrow { display: flex; align-items: center; justify-content: space-between; padding: 9px 14px; }
.langrow > span { color: #cfd2db; }

.add { display: flex; align-items: center; gap: 8px; margin: 4px 12px; padding: 8px 10px; width: calc(100% - 24px);
  background: rgba(255,255,255,0.05); border: 1px dashed rgba(255,255,255,0.18); border-radius: 9px; color: #bfe0ff; font-size: 12px; cursor: pointer; }

.delay { display: flex; align-items: center; justify-content: space-between; padding: 11px 14px; border-top: 1px solid rgba(255,255,255,0.08); }
.delay > span { color: #cfd2db; }

.stepper { display: flex; align-items: center; gap: 2px; background: rgba(255,255,255,0.06); border-radius: 8px; padding: 2px; }
.stepper button { width: 26px; height: 24px; border: none; background: none; color: #cfd2db; font-size: 15px; cursor: pointer; border-radius: 6px; }
.stepper .val { min-width: 46px; text-align: center; color: #fff; font-variant-numeric: tabular-nums; }
</style>
