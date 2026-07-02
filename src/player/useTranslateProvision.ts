import { useSettings } from './useSettings'
import { provisionTranslateEngine } from './backend'
import { useModelDownloads } from './useModelDownloads'

/** 字幕翻譯 master toggle 的備妥流程：ON → 若選定模型未下載則下載，成功才啟用。 */
export function useTranslateProvision() {
  async function requestEnable(): Promise<void> {
    const s = useSettings().state
    const key = s.liveSubs.translateModel
    const md = useModelDownloads()
    if (!md.downloaded.has(key)) {
      if (!md.downloading.get(key)) md.downloading.set(key, { done: 0, total: null })
      try {
        await provisionTranslateEngine(key)
      } catch {
        md.downloading.delete(key)
        return // 下載失敗 → 不啟用（toggle 維持 OFF）
      }
    }
    s.liveSubs.translateEnabled = true
  }
  function disable(): void {
    useSettings().state.liveSubs.translateEnabled = false
  }
  return { requestEnable, disable }
}
