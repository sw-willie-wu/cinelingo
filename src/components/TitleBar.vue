<script setup lang="ts">
import { computed } from 'vue'
import { usePlayer } from '../player/usePlayer'
import { useWindowControls } from '../player/useWindowControls'
import { displayTitle } from '../player/format'
import { maxIconKind } from '../player/windowIcons'
import PlayerIcon from './PlayerIcon.vue'
import { useSettings } from '../player/useSettings'

const player = usePlayer()
const { isMaximized, minimize, toggleMaximize, close } = useWindowControls()
const settings = useSettings()

const title = computed(() => {
  const src = player.source.current
  const remoteTitle = src?.kind === 'remote' ? src.resolved.title : null
  return displayTitle(remoteTitle, player.state.path)
})
const maxName = computed(() => maxIconKind(isMaximized.value))   // 'maximize' | 'restore'
</script>

<template>
  <div class="titlebar">
    <div class="title">{{ title }}</div>
    <div class="win-controls">
      <button class="wc" aria-label="設定" @click="settings.openModal()">
        <PlayerIcon name="settings" :size="17" />
      </button>
      <button class="wc" aria-label="最小化" @click="minimize()">
        <PlayerIcon name="minimize" :size="18" />
      </button>
      <button class="wc" aria-label="最大化還原" @click="toggleMaximize()">
        <PlayerIcon :name="maxName" :size="16" />
      </button>
      <button class="wc close" aria-label="關閉" @click="close()">
        <PlayerIcon name="close" :size="18" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  position: relative;
  display: flex; align-items: center; height: 36px;
  color: #fff; user-select: none;
}
.title {
  position: absolute; left: 50%; transform: translateX(-50%);
  max-width: 55%; text-align: center;
  font: 13px sans-serif; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.win-controls { display: flex; height: 100%; margin-left: auto; }
.wc {
  width: 44px; height: 100%; border: none; background: none; color: rgba(255, 255, 255, 0.88);
  cursor: pointer; display: flex; align-items: center; justify-content: center;
}
.wc:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
.wc.close:hover { background: #e81123; color: #fff; }
</style>
