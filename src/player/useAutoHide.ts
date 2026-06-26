import { ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { usePlayer } from './usePlayer'
import { shouldHide } from './autohide'

const IDLE_MS = 2500

export function useAutoHide() {
  const player = usePlayer()
  const visible = ref(true)
  const pointerOverBar = ref(false)
  let timer: ReturnType<typeof setTimeout> | null = null

  function clear() { if (timer) { clearTimeout(timer); timer = null } }

  function evaluate() {
    visible.value = !shouldHide({
      hasFile: player.state.path != null,
      pointerOverBar: pointerOverBar.value,
    })
  }

  function reset() {
    visible.value = true
    clear()
    timer = setTimeout(evaluate, IDLE_MS)
  }

  function onPointerMove() { reset() }

  function setPointerOverBar(over: boolean) {
    pointerOverBar.value = over
    if (over) { visible.value = true; clear() }   // 指標在控制列 → 常駐、停止倒數
    else reset()
  }

  // 載入新檔時重新評估(顯示 + 重新計時);暫停不再強制顯示(暫停也會隱藏,只有移動滑鼠/懸於列上才顯示)。
  watch(() => player.state.path, () => reset())

  onMounted(() => { window.addEventListener('pointermove', onPointerMove); reset() })
  onBeforeUnmount(() => { window.removeEventListener('pointermove', onPointerMove); clear() })

  return { visible, setPointerOverBar }
}
