<script setup lang="ts">
import { ref, watch } from 'vue'
import { useQueue } from '../player/useQueue'
import { useRecent } from '../player/useRecent'
import { usePlayer } from '../player/usePlayer'
import { expandPlayablePaths } from '../player/backend'
import { basename } from '../player/path'
import { REMOTE_TITLE_LOADING, type QueueItem } from '../player/queueTypes'
import type { RecentItem } from '../player/recentTypes'
import SidePanel from './SidePanel.vue'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ close: [] }>()

const queue = useQueue()
const recent = useRecent()
const player = usePlayer()
const tab = ref<'queue' | 'recent'>('queue')
const dragFrom = ref<number | null>(null)

// 每次開面板都重載 + 重判失效（load 有 once-guard；refreshMissing 抓 mid-session 刪檔，spec §4.3 lazy）。
watch(() => props.open, async (o) => {
  if (!o) return
  await recent.load()
  await recent.refreshMissing()
})

function onRecentClick(item: RecentItem) {
  if (item.missing) { player.notify('檔案已不存在'); return }   // spec §4.3/§6
  void queue.enqueueItems([{ kind: item.kind, id: item.id, title: item.title } as QueueItem], { interrupt: true })
}
function onDrop(i: number) {
  if (dragFrom.value !== null && dragFrom.value !== i) queue.move(dragFrom.value, i)
  dragFrom.value = null
}
async function onOpen() {
  const sel = await player.openFile()
  if (sel.length === 0) return
  let paths: string[]
  try { paths = await expandPlayablePaths(sel) }
  catch { player.notify('無法讀取選取的檔案'); return }
  if (paths.length === 0) { player.notify('沒有可播放的檔案'); return }
  await queue.enqueueItems(paths.map((p) => ({ kind: 'local', id: p, title: basename(p) })))
}
</script>

<template>
  <SidePanel :open="props.open" @close="emit('close')">
    <template #header>
      <div class="head">
        <div class="tabs">
          <button class="tab" :class="{ on: tab === 'queue' }" @click="tab = 'queue'">待播 ({{ queue.items.length }})</button>
          <button class="tab" :class="{ on: tab === 'recent' }" @click="tab = 'recent'">最近</button>
        </div>
        <button class="close" aria-label="關閉" @click="emit('close')">✕</button>
      </div>
    </template>

    <ul v-if="tab === 'queue'" class="rows">
      <li
        v-for="(it, i) in queue.items"
        :key="i"
        class="row"
        :class="{ current: i === queue.state.index }"
        draggable="true"
        @dragstart="dragFrom = i"
        @dragover.prevent
        @drop="onDrop(i)"
        @click="queue.playAt(i)"
      >
        <span class="idx">{{ i + 1 }}</span>
        <span v-if="it.title === REMOTE_TITLE_LOADING" class="spin" aria-hidden="true"></span>
        <span class="title">{{ it.title }}</span>
        <button class="x" aria-label="移除" @click.stop="queue.remove(i)">✕</button>
      </li>
      <li v-if="queue.items.length === 0" class="empty">拖入影片或貼上連結以開始</li>
    </ul>

    <ul v-else class="rows">
      <li
        v-for="it in recent.items"
        :key="it.id"
        class="row"
        :class="{ missing: it.missing }"
        @click="onRecentClick(it)"
      >
        <span class="dot" :class="it.kind">{{ it.kind === 'remote' ? '▷' : '◉' }}</span>
        <span class="title">{{ it.title }}</span>
        <button class="x" aria-label="移除" @click.stop="recent.remove(it.id)">✕</button>
      </li>
      <li v-if="recent.items.length === 0" class="empty">尚無最近播放</li>
    </ul>

    <template #footer>
      <div v-if="tab === 'queue'" class="foot">
        <button class="load" @click="onOpen">＋ 載入檔案…</button>
        <button v-if="queue.items.length" class="clear" @click="queue.clear()">清空</button>
      </div>
      <div v-else-if="recent.items.length" class="foot">
        <button class="clear full" @click="recent.clear()">清空最近</button>
      </div>
    </template>
  </SidePanel>
</template>

<style scoped>
.head { display: flex; align-items: center; gap: 6px; padding: 8px 8px 8px 6px; border-bottom: 1px solid rgba(255,255,255,0.08); }
.tabs { display: flex; gap: 4px; flex: 1; }
.tab { flex: 1; padding: 9px 0; border-radius: 9px; background: none; border: none; color: #b9b9c0; font-size: 13px; cursor: pointer; }
.tab.on { background: var(--accent); color: #fff; font-weight: 600; }
.close { width: 32px; height: 32px; border: none; background: none; color: #b9b9c0; font-size: 14px; cursor: pointer; border-radius: 8px; flex: none; }
.close:hover { color: #fff; background: rgba(255,255,255,0.1); }
.rows { list-style: none; margin: 0; padding: 6px; }
.row { display: flex; align-items: center; gap: 10px; padding: 10px; border-radius: 9px; cursor: pointer; border: 1px solid transparent; }
.row:hover { background: rgba(255,255,255,0.06); }
.row:hover .x { opacity: 1; }
.row.current { background: rgba(var(--accent-rgb),0.16); border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent) inset; }
.row.missing { opacity: .42; }
.idx { min-width: 18px; text-align: right; color: #8a8d99; font-variant-numeric: tabular-nums; }
.dot { width: 14px; text-align: center; color: #8a8d99; }
.title { flex: 1; min-width: 0; color: #dfe2ea; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.spin { width: 12px; height: 12px; flex: none; border: 2px solid rgba(255,255,255,0.25); border-top-color: var(--accent); border-radius: 50%; animation: qspin .7s linear infinite; }
@keyframes qspin { to { transform: rotate(360deg); } }
.x { opacity: 0; width: 24px; height: 24px; border: none; background: none; color: #b9b9c0; font-size: 13px; cursor: pointer; border-radius: 6px; flex: none; }
.x:hover { color: #fff; background: rgba(255,255,255,0.1); }
.empty { padding: 28px 14px; text-align: center; color: #8a8d99; cursor: default; line-height: 1.7; }
.foot { display: flex; gap: 8px; padding: 8px 10px; border-top: 1px solid rgba(255,255,255,0.08); }
.load { flex: 1; background: rgba(255,255,255,0.06); border: 1px dashed rgba(255,255,255,0.2); border-radius: 9px; color: #bfe0ff; font-size: 12px; padding: 8px 0; cursor: pointer; }
.load:hover { background: rgba(255,255,255,0.1); color: #fff; }
.clear { background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.16); border-radius: 9px; color: #cfd2db; font-size: 12px; padding: 8px 14px; cursor: pointer; }
.clear:hover { background: rgba(255,255,255,0.1); color: #fff; }
.clear.full { flex: 1; }
</style>
