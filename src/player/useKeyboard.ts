import { onMounted, onBeforeUnmount } from 'vue'
import { usePlayer } from './usePlayer'
import { useSettings } from './useSettings'
import { useEngineProvision } from './useEngineProvision'
import { useFloatingMode } from './useFloatingMode'

export function useKeyboard() {
  const player = usePlayer()
  const settings = useSettings()
  const prov = useEngineProvision()
  const floating = useFloatingMode()

  function onKeyDown(e: KeyboardEvent) {
    // 浮動字幕模式：Esc 退出，優先於 prov/modal（它們在後面會 early-return，故須最前）
    if (floating.active.value) {
      if (e.key === 'Escape') { e.preventDefault(); void floating.exit() }
      return
    }
    // 備妥對話框開啟：Esc 在 ask/error 關對話框、downloading 吞掉；一律不落到設定 modal。
    if (prov.state.open) {
      if (e.key === 'Escape') { e.preventDefault(); prov.cancel() }
      return
    }
    // modal 開啟：Esc 只關 modal，其餘全域快捷鍵不作用
    if (settings.modal.open) {
      if (e.key === 'Escape') { e.preventDefault(); settings.closeModal() }
      return
    }
    // 焦點在輸入控制項時不攔截（方向鍵等交給控制項）
    const tag = (e.target as HTMLElement | null)?.tagName
    if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') return

    switch (e.key) {
      case ' ': e.preventDefault(); player.togglePause(); break
      case 'ArrowLeft': player.seekBy(-5); break
      case 'ArrowRight': player.seekBy(5); break
      case 'ArrowUp': player.adjustVolume(5); break
      case 'ArrowDown': player.adjustVolume(-5); break
      case 'f': case 'F': player.toggleFullscreen(); break
      case 'Escape': player.exitFullscreen(); break
      case 'm': case 'M': player.toggleMute(); break
    }
  }

  onMounted(() => window.addEventListener('keydown', onKeyDown))
  onBeforeUnmount(() => window.removeEventListener('keydown', onKeyDown))
}
