<script setup lang="ts">
import { computed } from 'vue'
import { usePlayer } from '../../player/usePlayer'
import { useQueue } from '../../player/useQueue'
import { useSettings } from '../../player/useSettings'

const emit = defineEmits<{ back: [] }>()

const player = usePlayer()
const queue = useQueue()
const settings = useSettings()

const heights = computed<number[]>(() => {
  const cur = player.source.current
  if (!cur || cur.kind !== 'remote') return []
  const hs = cur.resolved.videos
    .map((v) => v.height)
    .filter((h): h is number => typeof h === 'number')
  return Array.from(new Set(hs)).sort((a, b) => b - a)
})

const options = computed<Array<'auto' | number>>(() => ['auto' as const, ...heights.value])

const current = computed(() => settings.state.youtube.quality)

function pick(q: 'auto' | number) {
  queue.reloadCurrentQuality(q)
  emit('back')
}
</script>

<template>
  <div>
    <div class="back" @click="$emit('back')">
      <span class="bk">‹</span><span class="ttl">畫質</span>
    </div>
    <ul class="srcs">
      <li
        v-for="opt in options"
        :key="opt"
        class="src"
        :class="{ sel: current === opt }"
        @click="pick(opt)"
      >
        <span class="label">{{ opt === 'auto' ? '自動' : `${opt}p` }}</span>
        <span v-if="current === opt" class="check">✓</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.back {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  cursor: pointer;
}
.bk { font-size: 16px; color: #cfd2db; }
.ttl { color: #fff; font-weight: 600; }

.srcs {
  list-style: none;
  margin: 0;
  padding: 6px;
}
.src {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 9px;
  cursor: pointer;
  border: 1px solid transparent;
}
.src:hover { background: rgba(255, 255, 255, 0.06); }
.src.sel {
  background: rgba(var(--accent-rgb), 0.16);
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent) inset;
}
.label { flex: 1; color: #e8e8ea; font-size: 13px; }
.check { color: var(--accent); font-size: 14px; font-weight: 700; }
</style>
