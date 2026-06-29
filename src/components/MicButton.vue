<script setup lang="ts">
import { ref } from 'vue'
import { usePlayer } from '../player/usePlayer'
import { useAudioSource } from '../player/useAudioSource'
import PlayerIcon from './PlayerIcon.vue'

const a = useAudioSource()
const player = usePlayer()
const confirmOpen = ref(false)

// 實際 arm 擷取：沿用上次來源；無上次 → 系統輸出；process 來源先重查清單(pid 可能更新)。
async function doArm() {
  const prev = a.current.value
  if (!prev) { void a.arm({ kind: 'system' }); return }
  if (prev.kind === 'process') {
    await a.refresh()
    const found = a.sources.value.processes.find((p) => p.name === prev.name)
    void a.arm(found ? { kind: 'process', name: found.name, pid: found.pid } : { kind: 'system' })
    return
  }
  void a.arm(prev)
}

function onClick() {
  if (a.armed.value) { void a.disarm(); return }          // 已擷取 → 關閉
  if (!player.isIdle.value) { confirmOpen.value = true; return }  // 播放中 → 互斥,先確認停播
  void doArm()                                             // 首頁 idle → 直接擷取
}

async function confirmCapture() {
  confirmOpen.value = false
  await player.closeMedia()   // 停止播放(互斥)
  void doArm()
}
</script>

<template>
  <div class="mic-wrap">
    <button
      class="btn"
      :class="{ armed: a.armed.value }"
      aria-label="外部音源"
      title="外部音源（擷取系統/應用程式聲音做即時字幕）"
      @click="onClick"
    >
      <PlayerIcon name="mic" :size="17" />
      <span v-if="a.armed.value" class="breath" />
    </button>

    <Teleport to="body">
      <Transition name="cf">
        <div v-if="confirmOpen" class="cf-backdrop" @click.self="confirmOpen = false">
          <div class="cf">
            <div class="cf-title">正在播放影片</div>
            <div class="cf-msg">要停止播放，改成擷取外部音源嗎？</div>
            <div class="cf-actions">
              <button class="cf-btn ghost" @click="confirmOpen = false">取消</button>
              <button class="cf-btn primary" @click="confirmCapture">開始擷取</button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.mic-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.btn {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 11px;
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.88);
  cursor: pointer;
  padding: 0;
  position: relative;
  transition: color 0.18s, filter 0.18s, transform 0.12s;
}
.btn:hover {
  color: #fff;
  filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.7)) drop-shadow(0 0 11px rgba(255, 255, 255, 0.42));
}
.btn.armed {
  color: var(--accent);
}
.btn.armed:hover {
  filter: drop-shadow(0 0 4px rgba(var(--accent-rgb), 0.8)) drop-shadow(0 0 12px rgba(var(--accent-rgb), 0.5));
}
.breath {
  position: absolute;
  top: 7px;
  right: 7px;
  width: 7px;
  height: 7px;
  background: var(--accent);
  border-radius: 50%;
  pointer-events: none;
  animation: breath 1.6s ease-in-out infinite;
}
@keyframes breath {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 1; }
}

/* 確認 modal（沿用 app 玻璃風） */
.cf-backdrop {
  position: fixed; inset: 0; z-index: 60;
  background: rgba(0, 0, 0, 0.45);
  display: flex; align-items: center; justify-content: center;
}
.cf {
  width: 320px; max-width: 88vw;
  background: rgba(26, 26, 30, 0.92);
  backdrop-filter: blur(28px) saturate(1.4); -webkit-backdrop-filter: blur(28px) saturate(1.4);
  border: 1px solid rgba(255, 255, 255, 0.12); border-radius: 14px;
  box-shadow: 0 24px 60px rgba(0, 0, 0, .65);
  color: #e8e8ea; padding: 20px; font: 13px/1.5 var(--font);
}
.cf-title { font-size: 15px; font-weight: 600; margin-bottom: 6px; }
.cf-msg { color: #b9b9c0; margin-bottom: 18px; }
.cf-actions { display: flex; gap: 8px; justify-content: flex-end; }
.cf-btn {
  padding: 8px 16px; border-radius: 9px; font: 13px var(--font); cursor: pointer; border: none;
  transition: background 0.15s, color 0.15s;
}
.cf-btn.ghost { background: rgba(255, 255, 255, 0.06); color: #cfd2db; border: 1px solid rgba(255, 255, 255, 0.14); }
.cf-btn.ghost:hover { background: rgba(255, 255, 255, 0.12); color: #fff; }
.cf-btn.primary { background: var(--accent); color: #fff; font-weight: 600; }
.cf-btn.primary:hover { filter: brightness(1.1); }

.cf-enter-active, .cf-leave-active { transition: opacity 0.18s ease; }
.cf-enter-from, .cf-leave-to { opacity: 0; }
</style>
