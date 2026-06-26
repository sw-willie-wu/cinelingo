// 取路徑最後一段（檔名）。支援 Windows 反斜線與正斜線。
export function basename(p: string): string {
  return p.split(/[\\/]/).pop() ?? p
}
