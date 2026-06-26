// Tauri startResizeDragging 的方向(拼字錯會靜默失效,故釘成型別 + 常數)
export type ResizeDir =
  | 'North' | 'South' | 'East' | 'West'
  | 'NorthEast' | 'NorthWest' | 'SouthEast' | 'SouthWest'

export interface ResizeHandleDef {
  key: string   // 對應 CSS class 後綴 rh-<key>
  dir: ResizeDir
}

export const RESIZE_HANDLES: readonly ResizeHandleDef[] = [
  { key: 'n', dir: 'North' },
  { key: 's', dir: 'South' },
  { key: 'e', dir: 'East' },
  { key: 'w', dir: 'West' },
  { key: 'ne', dir: 'NorthEast' },
  { key: 'nw', dir: 'NorthWest' },
  { key: 'se', dir: 'SouthEast' },
  { key: 'sw', dir: 'SouthWest' },
] as const
