/** 音訊 af 鏈純函式：10 段 EQ + 動態正規化。輸出 mpv `af` 屬性字串。 */
export const EQ_FREQS = [31, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000] as const

/** 各預設的 10 段增益（dB，±12）。手動拖動後 UI 標 'custom'，不在此表（custom 直接用 bands）。 */
export const EQ_PRESETS: Record<string, number[]> = {
  flat:    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  bass:    [6, 5, 4, 2, 0, 0, 0, 0, 0, 0],
  vocal:   [-2, -1, 0, 2, 4, 4, 3, 1, 0, -1],
  classic: [4, 3, 2, 0, 0, 0, -1, -1, 2, 3],
  rock:    [5, 4, 2, 0, -1, 0, 2, 3, 4, 4],
  jazz:    [3, 2, 1, 2, -1, -1, 0, 1, 2, 3],
}

export function buildEqBands(preset: string): number[] {
  return (EQ_PRESETS[preset] ?? EQ_PRESETS.flat).slice()
}

export interface AfInput { eq: { enabled: boolean; bands: number[] }; normalize: boolean }

export function buildAf(input: AfInput): string {
  const parts: string[] = []
  if (input.eq.enabled) {
    input.eq.bands.forEach((g, i) => {
      if (g !== 0 && i < EQ_FREQS.length) parts.push(`equalizer=f=${EQ_FREQS[i]}:t=o:w=1:g=${g}`)
    })
  }
  if (input.normalize) parts.push('dynaudnorm')
  return parts.join(',')
}
