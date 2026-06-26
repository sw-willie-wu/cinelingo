/** CC 切換的純邏輯：第二字幕相依 clamp、CC 還原來源挑選。 */
export interface CcSnapshot { primary: string; secondary: string }
interface FileRef { id: string }

/** primary 為 off 時 secondary 必為 off（單一 chokepoint 用）。 */
export function clampSecondaryToPrimary(primarySource: string, secondarySource: string): string {
  return primarySource === 'off' ? 'off' : secondarySource
}

function sourceValid(source: string, files: FileRef[], masterOn: boolean): boolean {
  if (source === 'off') return false
  if (source === 'live') return masterOn
  return files.some((f) => f.id === source)
}

/**
 * CC 開（兩軌皆 off）時挑要還原的來源。優先：有效快照 > 第一個字幕檔 > (master 開) live > null。
 * 回傳 { primary, secondary }（次軌僅在採用快照時保留並驗證，否則 off），或 null（無可顯示 → no-op）。
 */
export function pickCcRestore(snapshot: CcSnapshot | null, files: FileRef[], masterOn: boolean): CcSnapshot | null {
  if (snapshot && sourceValid(snapshot.primary, files, masterOn)) {
    const secondary = sourceValid(snapshot.secondary, files, masterOn) ? snapshot.secondary : 'off'
    return { primary: snapshot.primary, secondary }
  }
  if (files.length > 0) return { primary: files[0].id, secondary: 'off' }
  if (masterOn) return { primary: 'live', secondary: 'off' }
  return null
}
