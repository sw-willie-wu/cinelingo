export interface AutoHideInput {
  hasFile: boolean
  pointerOverBar: boolean
}

// 純判斷:有載入檔案 且 指標不在控制列上 → 可隱藏。
// (暫停也會隱藏;只有移動滑鼠或指標懸於控制列上才顯示。無檔時不隱藏。)
export function shouldHide(s: AutoHideInput): boolean {
  return s.hasFile && !s.pointerOverBar
}
