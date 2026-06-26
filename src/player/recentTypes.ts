export interface RecentItem {
  kind: 'local' | 'remote'
  id: string            // path 或 canonical watchUrl
  title: string
  lastPlayedAt: number  // Date.now()
  missing?: boolean      // 本地檔不存在（lazy 檢查）
}
