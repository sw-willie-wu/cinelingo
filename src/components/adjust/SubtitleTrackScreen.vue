<script setup lang="ts">
import { computed, watch } from 'vue'
import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useSubtitles } from '../../player/useSubtitles'
import { useSettings } from '../../player/useSettings'
import { useModelDownloads } from '../../player/useModelDownloads'
import { readTextFile } from '../../player/backend'
import { parseSubtitle } from '../../player/subtitles'
import { LANGS } from '../../player/langs'
import GlassSelect from '../GlassSelect.vue'
import PlayerIcon from '../PlayerIcon.vue'

const props = defineProps<{ track: 'primary' | 'secondary' }>()
const emit = defineEmits<{ back: [] }>()

const subs = useSubtitles()
const settings = useSettings()

const masterOn = computed(() => settings.state.liveSubs.enabled)
const cur = computed(() => subs.tracks[props.track])
const langOpts = LANGS.map((l) => ({ value: l.value, label: l.label }))
const md = useModelDownloads()
const translateReady = computed(() =>
  settings.state.liveSubs.translateEnabled && md.downloaded.has(settings.state.liveSubs.translateModel)
)
// 防呆：翻譯目標不列出來源語言（翻成同語言無意義；zh-Hant/zh-Hans 分開，故繁↔簡仍可）。
const transOpts = computed(() => {
  const src = settings.state.liveSubs.sourceLang
  return [
    { value: 'off', label: '關' },
    ...LANGS.filter((l) => l.value !== 'auto' && l.value !== src).map((l) => ({ value: l.value, label: l.label })),
  ]
})
// 已選目標剛好等於來源語言時歸零（避免顯示空白 / 白翻）。
watch(() => settings.state.liveSubs.sourceLang, (src) => {
  if (src !== 'auto' && cur.value.translateTo === src) subs.setTranslateTo(props.track, 'off')
})
// 翻譯成常駐顯示；有字幕來源（live 或字幕檔）+ 翻譯已啟用+模型就緒時可選，否則顯示原因提示。
const canTranslate = computed(() => cur.value.source !== 'off' && translateReady.value)
const transHint = computed(() =>
  cur.value.source === 'off' ? '需先選字幕來源' : '需在設定啟用翻譯'
)

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
      :class="{ sel: cur.source === 'off' }"
      @click="subs.selectSource(props.track, 'off')"
    >
      <span class="ic"><PlayerIcon name="ban" :size="18" /></span>關閉字幕
    </li>
    <li
      class="src"
      :class="{ sel: cur.source === 'live', disabled: !masterOn }"
      @click="masterOn && subs.selectSource(props.track, 'live')"
    >
      <span class="ic"><PlayerIcon name="captions-live" :size="18" /></span>即時字幕
      <span v-if="!masterOn" class="hint">需在設定開啟</span>
      <GlassSelect
        v-else
        class="langsel"
        :model-value="settings.state.liveSubs.sourceLang"
        :options="langOpts"
        @update:model-value="settings.state.liveSubs.sourceLang = $event"
        @click.stop
      />
    </li>
    <li
      v-for="f in subs.files.value"
      :key="f.id"
      class="src"
      :class="{ sel: cur.source === f.id }"
      @click="subs.selectSource(props.track, f.id)"
    >
      <span class="ic"><PlayerIcon name="file" :size="18" /></span><span class="fname">{{ f.name }}</span>
    </li>
    <li class="src add-src" @click="addFile">
      <span class="ic"><PlayerIcon name="plus" :size="18" /></span>新增字幕檔…
    </li>
  </ul>

  <div class="transrow" :class="{ dim: !canTranslate, sel: canTranslate && cur.translateTo !== 'off' }">
    <span class="ic"><PlayerIcon name="translate" :size="18" /></span>
    <span>翻譯成</span>
    <GlassSelect
      v-if="canTranslate"
      class="langsel"
      :model-value="cur.translateTo"
      :options="transOpts"
      @update:model-value="subs.setTranslateTo(props.track, $event)"
    />
    <span v-else class="hint">{{ transHint }}</span>
  </div>

  <div class="delay">
    <span class="ic"><PlayerIcon name="audiodelay" :size="18" /></span>
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
.src { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 9px; cursor: pointer; }
.src::before { content: '✓'; flex-shrink: 0; width: 12px; font-size: 12px; color: var(--accent); visibility: hidden; }
.src.sel::before { visibility: visible; }
.src:hover { background: rgba(255,255,255,0.06); }
.src.disabled { opacity: .42; cursor: not-allowed; }
.src.disabled:hover { background: none; }
.hint { margin-left: auto; font-size: 10px; color: #8a8d99; }
.langsel { margin-left: auto; }
.ic { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; color: #cfd2db; width: 18px; height: 18px; }
.fname { color: #dfe2ea; word-break: break-all; }
.add-src { color: #bfe0ff; }
.add-src .ic { color: #bfe0ff; }

.transrow { display: flex; align-items: center; gap: 10px; padding: 11px 18px 9px; border-top: 1px solid rgba(255,255,255,0.08); }
.transrow::before { content: '✓'; flex-shrink: 0; width: 12px; font-size: 12px; color: var(--accent); visibility: hidden; }
.transrow.sel::before { visibility: visible; }
.transrow > span { color: #cfd2db; }
.transrow.dim > span { color: #6b6e7a; }

.delay { display: flex; align-items: center; gap: 10px; padding: 9px 18px 11px; }
.delay::before { content: ''; flex-shrink: 0; width: 12px; }
.delay > span { color: #cfd2db; }

.stepper { display: flex; align-items: center; gap: 2px; margin-left: auto; background: rgba(255,255,255,0.06); border-radius: 8px; padding: 2px; }
.stepper button { width: 26px; height: 24px; border: none; background: none; color: #cfd2db; font-size: 15px; cursor: pointer; border-radius: 6px; }
.stepper .val { min-width: 46px; text-align: center; color: #fff; font-variant-numeric: tabular-nums; }
</style>
