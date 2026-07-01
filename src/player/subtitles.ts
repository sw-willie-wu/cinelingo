export interface Cue {
  id: string
  sessionId: string
  startSec: number
  endSec: number
  sourceText: string
  lang: string | null
  status: 'final' | 'interim'
  targetText?: string
  targetLang?: string
}

/** 決定性鍵：來源段絕對起始毫秒（同段重轉得同 id → upsertCue 可取代）。 */
export function deriveCueId(startMs: number): string {
  return String(startMs)
}

/** 以 id 取代或插入，維持依 startSec 升冪。 */
export function upsertCue(list: Cue[], c: Cue): Cue[] {
  const next = list.filter((x) => x.id !== c.id)
  next.push(c)
  next.sort((a, b) => a.startSec - b.startSec)
  return next
}

/** 批次 upsert（種子快取一次灌入；避免逐條 O(n) 插入造成 ~O(n²)）。維持依 startSec 升冪。 */
export function upsertCues(list: Cue[], incoming: Cue[]): Cue[] {
  if (incoming.length === 0) return [...list].sort((a, b) => a.startSec - b.startSec)
  const ids = new Set(incoming.map((c) => c.id))
  const next = list.filter((x) => !ids.has(x.id))
  next.push(...incoming)
  next.sort((a, b) => a.startSec - b.startSec)
  return next
}

/** 時間落在 [start,end) 的第一條（純時間，不看 sessionId；每個來源各自獨立 cue 陣列）。 */
export function selectCueAt(cues: Cue[], timeSec: number): Cue | null {
  for (const c of cues) if (timeSec >= c.startSec && timeSec < c.endSec) return c
  return null
}

// ── no-clock 顯示常數（spec §5）──
export const MAX_PHRASE_CHARS = 16   // 每行字數 fallback（視窗寬/字型未知時；displayCharCap 算不出時的退路）
export const DISPLAY_FINALS_SCAN = 12
export const FINALS_CAP = 5000
export const LIVE_WIDTH_FRACTION = 0.70 // 即時字幕一行最多佔視窗寬的比例

/** 依視窗寬與字型 px 算「每行字數上限」（CJK 全形 ≈ 1em 寬；一行佔 ≤ fraction 視窗寬）。
 *  fraction 預設 LIVE_WIDTH_FRACTION（overlay 傳使用者設定的 maxWidthPct/100）。
 *  下限 8 字、寬度/字型/比例無效時退回 MAX_PHRASE_CHARS。純函式。 */
export function displayCharCap(winWidthPx: number, fontPx: number, fraction: number = LIVE_WIDTH_FRACTION): number {
  if (!(winWidthPx > 0) || !(fontPx > 0) || !(fraction > 0)) return MAX_PHRASE_CHARS
  return Math.max(8, Math.floor((winWidthPx * fraction) / fontPx))
}

function isCjk(c: string): boolean {
  const cp = c.codePointAt(0)
  if (cp === undefined) return false
  return (cp >= 0x4e00 && cp <= 0x9fff) || (cp >= 0x3400 && cp <= 0x4dbf)
    || (cp >= 0x3040 && cp <= 0x30ff) || (cp >= 0x3000 && cp <= 0x303f)
    || (cp >= 0xff00 && cp <= 0xffef)
}

/** 依「CJK 字後空白」拆 phrase（拉丁詞間空白保留同行）；移植自後端 split_cjk_phrases。 */
function splitCjkPhrases(text: string): string[] {
  const rows: string[] = []
  let cur = ''
  let prevNonSpace: string | null = null
  for (const c of text) {
    if (/\s/.test(c)) {
      if (prevNonSpace !== null && isCjk(prevNonSpace)) {
        const t = cur.trim(); if (t) rows.push(t); cur = ''; prevNonSpace = null
      } else { cur += c }
    } else { cur += c; prevNonSpace = c }
  }
  const t = cur.trim(); if (t) rows.push(t)
  return rows
}

/** 把一行硬斷成 ≤cap（CJK 字界 / 拉丁詞界 / 單一超長拉丁詞硬切）。決定性、每行 ≤cap。 */
export function hardWrap(s: string, cap: number): string[] {
  cap = Math.max(1, cap)
  const out: string[] = []
  let line = ''
  for (const ch of s) {
    if ([...(line + ch)].length <= cap) { line += ch; continue }
    const lastSpace = line.lastIndexOf(' ')
    const word = lastSpace >= 0 ? line.slice(lastSpace + 1) : line
    const wordIsLatin = word.length > 0 && ![...word].some(isCjk) && !/\s/.test(ch)
    if (lastSpace >= 0 && wordIsLatin) {
      const head = line.slice(0, lastSpace).trimEnd()
      if (head) out.push(head)
      line = line.slice(lastSpace + 1) + ch
    } else {
      const t = line.trim(); if (t) out.push(t)
      line = /\s/.test(ch) ? '' : ch
    }
  }
  const t = line.trim(); if (t) out.push(t)
  return out
}

/** 辨識中(interim)用：CJK 空白拆 phrase + 每 phrase 硬斷 ≤cap（靠寬度截斷；overlay 傳動態 displayCharCap）。 */
export function splitDisplayPhrases(text: string, cap: number = MAX_PHRASE_CHARS): string[] {
  const out: string[] = []
  for (const p of splitCjkPhrases(text)) for (const line of hardWrap(p, cap)) out.push(line)
  return out
}

// 句末標點（強斷句）；逗號/頓號不斷。
const SENTENCE_ENDERS = '。．！？!?；;'
/** 定稿(final)用：「正常分行」——在句末標點後斷行（標點留行末）、trim、去空；無標點則整段一行。
 *  不靠寬度截斷（過長交給 CSS soft-wrap，屬正常字幕行為）。純函式。 */
export function splitFinalLines(text: string): string[] {
  const out: string[] = []
  let cur = ''
  for (const ch of text) {
    cur += ch
    if (SENTENCE_ENDERS.includes(ch)) { const t = cur.trim(); if (t) out.push(t); cur = '' }
  }
  const t = cur.trim(); if (t) out.push(t)
  return out
}

/** no-clock 多行顯示：finals 走「正常分行」(splitFinalLines、句末標點、不截斷)、interim 走「寬度截斷」
 *  (splitDisplayPhrases、cap)，合併取「最後 N 行」滾動視窗——interim 置底且優先保留（再長也只佔 ≤N 行、
 *  把舊 final 行擠掉），故 lines+interimLines 總數恆 ≤ n。cap 只作用於 interim。 */
export function liveLines(
  finals: readonly Cue[],
  interim: Cue | null,
  n: number,
  cap: number = MAX_PHRASE_CHARS,
): { lines: string[]; interimLines: string[] } {
  const scan = finals.slice(-DISPLAY_FINALS_SCAN)
  const phrases: string[] = []
  for (const c of scan) for (const p of splitFinalLines(c.sourceText)) phrases.push(p)
  const dedup: string[] = []
  for (const p of phrases) if (dedup[dedup.length - 1] !== p) dedup.push(p)
  const allInterim = interim && interim.sourceText.trim() !== '' ? splitDisplayPhrases(interim.sourceText, cap) : []
  // interim 優先：先放 interim（置底、最多 n 行），剩餘額度給最新的 final 行。
  const interimLines = n > 0 ? allInterim.slice(-n) : []
  const remaining = n - interimLines.length
  const lines = remaining > 0 ? dedup.slice(-remaining) : [] // 注意：slice(-0)===slice(0) 會回全部，故需 remaining>0 守衛
  return { lines, interimLines }
}

export interface LiveBlock { id: string; sourceLines: string[]; target?: string; interim?: boolean }

/** translate 模式的結構化渲染：以 cue 為單位（保留 id/targetText），取最後 n 條 final + 末端 interim。 */
export function liveBlocks(
  finals: readonly Cue[],
  interim: Cue | null,
  n: number,
  cap: number = MAX_PHRASE_CHARS,
): LiveBlock[] {
  const out: LiveBlock[] = []
  const recent = n > 0 ? finals.slice(-n) : []
  for (const c of recent) {
    out.push({ id: c.id, sourceLines: splitFinalLines(c.sourceText), target: c.targetText })
  }
  if (interim && interim.sourceText.trim() !== '') {
    out.push({ id: interim.id, sourceLines: splitDisplayPhrases(interim.sourceText, cap), interim: true })
  }
  return out
}

// 去除常見行內標記：HTML 樣式標籤、VTT 的 <v ...>/<c...>、SRT 的 {\anX} 等定位碼。
function stripMarkup(s: string): string {
  return s.replace(/<[^>]+>/g, '').replace(/\{\\[^}]*\}/g, '').trim()
}

// "HH:MM:SS,mmm" / "HH:MM:SS.mmm" / "MM:SS.mmm" → 秒。失敗回 null。
function timeToSec(s: string): number | null {
  const m = s.trim().match(/^(?:(\d+):)?(\d{1,2}):(\d{2})[.,](\d{1,3})$/)
  if (!m) return null
  const [, h, mm, ss, ms] = m
  return (h ? +h * 3600 : 0) + +mm * 60 + +ss + +ms.padEnd(3, '0') / 1000
}

// 共用：把「時間行 --> 時間行」+ 後續文字行的區塊轉 Cue（idx 給 id）。
function blockToCue(timeLine: string, textLines: string[], idx: number): Cue | null {
  const m = timeLine.match(/(\S+)\s*-->\s*(\S+)/)
  if (!m) return null
  const start = timeToSec(m[1]), end = timeToSec(m[2])
  if (start == null || end == null) return null
  const sourceText = stripMarkup(textLines.join('\n'))
  return { id: String(idx), sessionId: '', startSec: start, endSec: end, sourceText, lang: null, status: 'final' }
}

export function parseSrt(text: string): Cue[] {
  const out: Cue[] = []
  for (const block of text.replace(/\r\n/g, '\n').split(/\n{2,}/)) {
    const lines = block.split('\n').filter((l) => l.trim() !== '')
    if (lines.length < 2) continue
    // 第一行可能是序號；找含 "-->" 的時間行。
    const ti = lines.findIndex((l) => l.includes('-->'))
    if (ti < 0) continue
    const cue = blockToCue(lines[ti], lines.slice(ti + 1), out.length)
    if (cue) out.push(cue)
  }
  return out
}

export function parseVtt(text: string): Cue[] {
  const out: Cue[] = []
  for (const block of text.replace(/\r\n/g, '\n').replace(/^﻿/, '').split(/\n{2,}/)) {
    const trimmed = block.trim()
    if (trimmed === '' || trimmed.startsWith('WEBVTT') || trimmed.startsWith('NOTE')) continue
    const lines = block.split('\n').filter((l) => l.trim() !== '')
    const ti = lines.findIndex((l) => l.includes('-->'))
    if (ti < 0) continue
    const cue = blockToCue(lines[ti], lines.slice(ti + 1), out.length)
    if (cue) out.push(cue)
  }
  return out
}

/** 依副檔名路由至正確解析器（.vtt → parseVtt；其餘 → parseSrt）。 */
export function parseSubtitle(name: string, text: string): Cue[] {
  return name.toLowerCase().endsWith('.vtt') ? parseVtt(text) : parseSrt(text)
}
