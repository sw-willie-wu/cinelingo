<script setup lang="ts">
import { onMounted } from 'vue'
import { useAudioSource } from '../../player/useAudioSource'
import PlayerIcon from '../PlayerIcon.vue'

const emit = defineEmits<{ back: [] }>()
const a = useAudioSource()

onMounted(() => a.refresh())

function pick(sel: Parameters<typeof a.arm>[0]) {
  void a.arm(sel)
}
function off() {
  if (a.armed.value) void a.disarm()
}
</script>

<template>
  <div class="back" @click="emit('back')">
    <span class="bk">‹</span>
    <span class="ttl">外部音源</span>
  </div>

  <div class="lbl">音源</div>
  <ul class="srcs">
    <li
      class="src"
      :class="{ sel: !a.armed.value }"
      @click="off"
    >
      <span class="ic"><PlayerIcon name="ban" :size="18" /></span>關閉擷取
    </li>
    <li
      class="src"
      :class="{ sel: a.armed.value && a.current.value?.kind === 'system' }"
      @click="pick({ kind: 'system' })"
    >
      <span class="ic"><PlayerIcon name="volume" :size="18" /></span>系統輸出（全部聲音）
    </li>
  </ul>

  <template v-if="a.sources.value.processes.length">
    <div class="lbl">正在播音的程式</div>
    <ul class="srcs">
      <li
        v-for="p in a.sources.value.processes"
        :key="p.pid"
        class="src"
        :class="{ sel: a.armed.value && a.current.value?.kind === 'process' && a.current.value.name === p.name }"
        @click="pick({ kind: 'process', name: p.name, pid: p.pid })"
      >
        <span class="ic">
          <img v-if="p.icon" :src="p.icon" class="app-icon" alt="" />
          <PlayerIcon v-else name="grid" :size="18" />
        </span>{{ p.name }}
      </li>
    </ul>
  </template>

  <template v-if="a.sources.value.inputDevices.length">
    <div class="lbl">麥克風 / 輸入裝置</div>
    <ul class="srcs">
      <li
        v-for="d in a.sources.value.inputDevices"
        :key="d.id"
        class="src"
        :class="{ sel: a.armed.value && a.current.value?.kind === 'inputDevice' && a.current.value.id === d.id }"
        @click="pick({ kind: 'inputDevice', id: d.id })"
      >
        <span class="ic"><PlayerIcon name="mic" :size="18" /></span>{{ d.name }}
      </li>
    </ul>
  </template>
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
.ic { display: inline-flex; align-items: center; justify-content: center; flex-shrink: 0; color: #cfd2db; width: 18px; height: 18px; }
.app-icon { width: 18px; height: 18px; object-fit: contain; }
</style>
