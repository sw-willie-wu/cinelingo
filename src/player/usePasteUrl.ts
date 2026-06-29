import { onMounted, onBeforeUnmount } from 'vue'
import { isHttpUrl, playlistContext, canonicalRemoteId } from './remoteUrl'
import { usePlayer } from './usePlayer'
import { useQueue } from './useQueue'
import { useSettings } from './useSettings'
import { checkYtdlp, enumeratePlaylist, remoteTitle } from './backend'
import { REMOTE_TITLE_LOADING, type QueueItem } from './queueTypes'
import { useAudioSource } from './useAudioSource'

// 全域 paste：焦點在輸入框 → 放行原生貼上；否則任意 http(s) URL → 嘗試載入（交 yt-dlp 解析）。
// 開關未啟用 / yt-dlp 未備妥 / 解析失敗都給可見提示（player.notify → 畫面中央 toast，~5s 淡掉），不靜默。
export function usePasteUrl() {
  const player = usePlayer()
  const queue = useQueue()
  const settings = useSettings()
  async function onPaste(e: ClipboardEvent) {
    const t = e.target as HTMLElement | null
    const tag = t?.tagName
    if (tag === 'INPUT' || tag === 'TEXTAREA' || t?.isContentEditable) return // 放行原生貼上
    const text = (e.clipboardData?.getData('text') ?? '').trim() // 同步讀（在 await 前）
    if (!isHttpUrl(text)) return                       // 非 URL → 不攔，放行原生貼上
    e.preventDefault()                                  // 是 URL → 由我們處理（同步攔截，在 await 前才有效）
    if (!settings.state.capture.enabled) {
      player.notify('請先到「設定 → 擷取與錄製」啟用「YouTube／網路來源」')
      return
    }
    if (!(await checkYtdlp())) {
      player.notify('解析工具（yt-dlp）尚未備妥，請到設定重新啟用')
      return
    }
    const ctx = playlistContext(text)
    if (ctx) {
      let fp
      try { fp = await enumeratePlaylist(text) } catch { player.notify('無法讀取此清單'); return }
      if (fp.entries.length === 0) { player.notify('清單是空的或無法讀取'); return }
      const items: QueueItem[] = fp.entries.map((en) => ({
        kind: 'remote', id: `https://www.youtube.com/watch?v=${en.id}`, title: en.title,
      }))
      // 進入點：watch?v=ID&list → ID 在列 index；純清單 → 0；不在列 → 0。僅空佇列時生效（Q1）。
      const off = ctx.videoId ? Math.max(0, fp.entries.findIndex((en) => en.id === ctx.videoId)) : 0
      await queue.enqueueItems(items, { startOffset: off, noAutoplay: useAudioSource().armed.value })
    } else {
      // 單片：id 必 canonicalize（與 loadUrl 一致）→ 同片不同 URL 形式共用字幕記憶/快取/最近去重鍵。
      // 先進清單暫顯「讀取中…」（QueuePanel 依此哨兵顯 spinner），背景輕量抓標題回填；失敗回退原 URL。
      const item: QueueItem = { kind: 'remote', id: canonicalRemoteId(text), title: REMOTE_TITLE_LOADING }
      await queue.enqueueItems([item], { noAutoplay: useAudioSource().armed.value })
      // 背景輕量抓標題；經 queue.backfillTitle 透過 reactive proxy 寫入（直接 mutate item 原物件不具反應性、
      // UI 不會更新）。守衛（只改仍是哨兵的同 id 項）在 backfillTitle 內 → 不覆寫 playAt 已解析的真標題。
      remoteTitle(item.id)
        .then((t) => queue.backfillTitle(item.id, t || text))
        .catch(() => queue.backfillTitle(item.id, text))
    }
  }
  onMounted(() => window.addEventListener('paste', onPaste))
  onBeforeUnmount(() => window.removeEventListener('paste', onPaste))
}
