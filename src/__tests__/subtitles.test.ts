import { describe, it, expect } from 'vitest'
import { deriveCueId, upsertCue, upsertCues, parseSrt, parseVtt, parseSubtitle, selectCueAt, hardWrap, splitDisplayPhrases, splitFinalLines, liveLines, displayCharCap, MAX_PHRASE_CHARS, type Cue } from '../player/subtitles'

const cue = (o: Partial<Cue>): Cue => ({
  id: 'x',
  sessionId: 's1',
  startSec: 0,
  endSec: 1,
  sourceText: 't',
  lang: null,
  status: 'final',
  ...o,
})

describe('deriveCueId', () => {
  it('deterministic from startMs', () => {
    expect(deriveCueId(1500)).toBe(deriveCueId(1500))
  })
  it('differs by startMs', () => {
    expect(deriveCueId(1500)).not.toBe(deriveCueId(1600))
  })
})

describe('upsertCue', () => {
  it('appends sorted', () => {
    let l: Cue[] = []
    l = upsertCue(l, cue({ id: 'a', startSec: 2 }))
    l = upsertCue(l, cue({ id: 'b', startSec: 1 }))
    expect(l.map((c) => c.id)).toEqual(['b', 'a'])
  })
  it('replaces same id', () => {
    let l = upsertCue([], cue({ id: 'a', sourceText: 'old' }))
    l = upsertCue(l, cue({ id: 'a', sourceText: 'new' }))
    expect(l).toHaveLength(1)
    expect(l[0].sourceText).toBe('new')
  })
})

describe('upsertCues', () => {
  it('bulk inserts sorted and replaces by id', () => {
    let l = upsertCue([], cue({ id: 'a', startSec: 5, sourceText: 'old' }))
    l = upsertCues(l, [cue({ id: 'b', startSec: 1 }), cue({ id: 'a', startSec: 5, sourceText: 'new' })])
    expect(l.map((c) => c.id)).toEqual(['b', 'a'])
    expect(l.find((c) => c.id === 'a')?.sourceText).toBe('new')
  })
  it('empty incoming returns sorted original', () => {
    const l = upsertCues([cue({ id: 'a', startSec: 2 })], [])
    expect(l.map((c) => c.id)).toEqual(['a'])
  })
})

describe('selectCueAt', () => {
  const cues = [
    { id: '0', sessionId: '', startSec: 1, endSec: 2, sourceText: 'A', lang: null, status: 'final' as const },
    { id: '1', sessionId: '', startSec: 5, endSec: 6, sourceText: 'B', lang: null, status: 'final' as const },
  ]
  it('hits within [start,end)', () => { expect(selectCueAt(cues, 1.5)?.sourceText).toBe('A') })
  it('end is exclusive', () => { expect(selectCueAt(cues, 2)).toBeNull() })
  it('gap → null', () => { expect(selectCueAt(cues, 3)).toBeNull() })
  it('empty → null', () => { expect(selectCueAt([], 1)).toBeNull() })
})

describe('parseSrt', () => {
  it('parses blocks, strips markup, handles CRLF', () => {
    const txt = '1\r\n00:00:01,000 --> 00:00:02,500\r\n<i>Hello</i>\r\n\r\n2\n00:01:00,000 --> 00:01:02,000\n{\\an8}World\nLine2\n'
    const c = parseSrt(txt)
    expect(c.length).toBe(2)
    expect(c[0]).toMatchObject({ startSec: 1, endSec: 2.5, sourceText: 'Hello' })
    expect(c[1]).toMatchObject({ startSec: 60, endSec: 62, sourceText: 'World\nLine2' })
  })
  it('ignores malformed blocks', () => { expect(parseSrt('garbage\nno timing here\n')).toEqual([]) })
})

describe('parseVtt', () => {
  it('parses WEBVTT, hours-optional, strips tags/NOTE/settings', () => {
    const txt = 'WEBVTT\n\nNOTE skip me\n\n00:01.000 --> 00:02.000 align:start\n<v Bob>Hi</v>\n\n00:00:05.000 --> 00:00:06.000\n<c.yellow>There</c>\n'
    const c = parseVtt(txt)
    expect(c.length).toBe(2)
    expect(c[0]).toMatchObject({ startSec: 1, endSec: 2, sourceText: 'Hi' })
    expect(c[1]).toMatchObject({ startSec: 5, endSec: 6, sourceText: 'There' })
  })
})

describe('parseSubtitle', () => {
  // Discriminating input: a VTT block with ONLY a timing line and no text.
  // parseSrt (line 59) drops it because lines.length < 2 → 0 cues.
  // parseVtt has no such check; it only skips WEBVTT/NOTE blocks → 1 cue (sourceText: '').
  // This makes parseSrt and parseVtt produce provably different results on this input.
  const discriminatingVtt = 'WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n'

  it('routes .vtt name to parseVtt (not parseSrt)', () => {
    expect(parseSubtitle('subs.vtt', discriminatingVtt)).toEqual(parseVtt(discriminatingVtt))
    expect(parseSubtitle('subs.vtt', discriminatingVtt)).not.toEqual(parseSrt(discriminatingVtt))
  })

  it('routes .srt name to parseSrt (not parseVtt)', () => {
    expect(parseSubtitle('subs.srt', discriminatingVtt)).toEqual(parseSrt(discriminatingVtt))
    expect(parseSubtitle('subs.srt', discriminatingVtt)).not.toEqual(parseVtt(discriminatingVtt))
  })

  it('is case-insensitive on extension (.VTT routes to parseVtt)', () => {
    expect(parseSubtitle('subs.VTT', discriminatingVtt)).toEqual(parseVtt(discriminatingVtt))
    expect(parseSubtitle('subs.VTT', discriminatingVtt)).not.toEqual(parseSrt(discriminatingVtt))
  })
})

describe('hardWrap', () => {
  it('CJK 無空白超長 → 字界硬切 ≤cap', () => {
    expect(hardWrap('一二三四五', 2)).toEqual(['一二', '三四', '五'])
  })
  it('拉丁詞界優先（≤cap 的最後空白斷）', () => {
    expect(hardWrap('hello world foo', 8)).toEqual(['hello', 'world', 'foo'])
  })
  it('單一超長拉丁詞 → 硬切', () => {
    expect(hardWrap('supercalifragilistic', 8)).toEqual(['supercal', 'ifragili', 'stic'])
  })
  it('CJK+拉丁混排 ≤cap', () => {
    const out = hardWrap('這是abcdefghij', 6)
    out.forEach((l) => expect([...l].length).toBeLessThanOrEqual(6))
    expect(out.join('')).toContain('這是')
  })
  it('短於 cap 原樣、空字串空陣列', () => {
    expect(hardWrap('你好', 18)).toEqual(['你好'])
    expect(hardWrap('', 18)).toEqual([])
  })
})

describe('splitDisplayPhrases', () => {
  it('CJK 字後空白拆 phrase', () => {
    expect(splitDisplayPhrases('看看背景 公館市場')).toEqual(['看看背景', '公館市場'])
  })
  it('拉丁詞間空白不拆（同行）', () => {
    expect(splitDisplayPhrases('I see you')).toEqual(['I see you'])
  })
  it('每行 ≤ MAX_PHRASE_CHARS', () => {
    const long = '一二三四五六七八九十一二三四五六七八九十一二三四五' // 25 CJK
    splitDisplayPhrases(long).forEach((l) => expect([...l].length).toBeLessThanOrEqual(MAX_PHRASE_CHARS))
  })
})

const fin = (id: string, start: number, text: string): Cue =>
  ({ id, sessionId: 's', startSec: start, endSec: start + 1, sourceText: text, lang: 'zh', status: 'final' })
const interimCue = (text: string): Cue =>
  ({ id: 's:interim', sessionId: 's', startSec: 99, endSec: 100, sourceText: text, lang: 'zh', status: 'interim' })
// 產生 n 個相異、無空白的 CJK 字（讓 hardWrap/liveLines 測試不綁死 MAX_PHRASE_CHARS 的值）。
const cjk = (n: number): string => Array.from({ length: n }, (_, i) => String.fromCodePoint(0x4e00 + i)).join('')

describe('liveLines', () => {
  it('finals + interim 合計 ≤ n、interim 置底（擠掉最舊 final）', () => {
    const finals = [fin('1', 1, '第一句'), fin('2', 2, '第二句'), fin('3', 3, '第三句'), fin('4', 4, '第四句')]
    const r = liveLines(finals, interimCue('正在講'), 3)
    expect(r.interimLines).toEqual(['正在講'])
    expect(r.lines).toEqual(['第三句', '第四句']) // 3 - 1(interim) = 最新 2 條 final
    expect(r.lines.length + r.interimLines.length).toBe(3)
  })
  it('相鄰去重', () => {
    const finals = [fin('1', 1, '對'), fin('2', 2, '對'), fin('3', 3, '好')]
    const r = liveLines(finals, null, 3)
    expect(r.lines).toEqual(['對', '好'])
  })
  it('null/空 interim → 無 interim 行', () => {
    expect(liveLines([fin('1', 1, 'x')], null, 3).interimLines).toEqual([])
    expect(liveLines([fin('1', 1, 'x')], interimCue('  '), 3).interimLines).toEqual([])
  })
  it('final 句末標點正常分行後取 last-N', () => {
    const finals = [fin('1', 1, '一二三四五。六七八九十。甲乙丙丁戊。')]
    const r = liveLines(finals, null, 2)
    expect(r.lines).toEqual(['六七八九十。', '甲乙丙丁戊。'])
  })
  it('interim 佔部分 N 行、final 留剩餘額度（總數 = n）', () => {
    const finals = [fin('1', 1, '舊一'), fin('2', 2, '舊二')]
    // 長度 = cap+1 的無空白 CJK → 硬斷成 2 行 interim（與 cap 實際值無關）
    const r = liveLines(finals, interimCue(cjk(MAX_PHRASE_CHARS + 1)), 3)
    expect(r.interimLines.length).toBe(2)
    expect(r.lines).toEqual(['舊二']) // 3 - 2(interim) = 最新 1 條 final
    expect(r.lines.length + r.interimLines.length).toBe(3)
    r.interimLines.forEach((l) => expect([...l].length).toBeLessThanOrEqual(MAX_PHRASE_CHARS))
  })
  it('interim ≥ n 行 → 佔滿 n、final 全擠掉（防 slice(-0) 回全部）', () => {
    const finals = [fin('1', 1, '舊'), fin('2', 2, '掉')]
    // 長度 = cap*3+1 → 硬斷成 4 行；slice(-3) 取最後 3 行（= n）
    const r = liveLines(finals, interimCue(cjk(MAX_PHRASE_CHARS * 3 + 1)), 3)
    expect(r.interimLines.length).toBe(3)
    expect(r.lines).toEqual([]) // 不可回全部 final
    r.interimLines.forEach((l) => expect([...l].length).toBeLessThanOrEqual(MAX_PHRASE_CHARS))
  })
  it('cap 只截斷 interim（final 不受 cap 影響）', () => {
    // interim：cap=5、20 CJK 無標點 → 4 行（受寬度截斷）
    const ri = liveLines([], interimCue(cjk(20)), 5, 5)
    expect(ri.interimLines.length).toBe(4)
    ri.interimLines.forEach((l) => expect([...l].length).toBeLessThanOrEqual(5))
    // final：無句末標點 → 整段一行（不被 cap 截斷）
    const rf = liveLines([fin('1', 1, cjk(20))], null, 5, 5)
    expect(rf.lines).toEqual([cjk(20)])
  })
})

describe('displayCharCap', () => {
  it('依視窗寬 × 字型 px 算（佔 70%）', () => {
    expect(displayCharCap(1920, 28)).toBe(Math.floor(1920 * 0.7 / 28)) // 48
    expect(displayCharCap(800, 40)).toBe(14)
  })
  it('下限 8 字', () => {
    expect(displayCharCap(100, 40)).toBe(8) // floor(70/40)=1 → 8
  })
  it('寬度/字型無效 → 退回 MAX_PHRASE_CHARS', () => {
    expect(displayCharCap(0, 28)).toBe(MAX_PHRASE_CHARS)
    expect(displayCharCap(1920, 0)).toBe(MAX_PHRASE_CHARS)
  })
})

describe('splitFinalLines', () => {
  it('句末標點後斷行、標點留行末', () => {
    expect(splitFinalLines('甲乙丙。丁戊！')).toEqual(['甲乙丙。', '丁戊！'])
    expect(splitFinalLines('問號？驚嘆!分號;')).toEqual(['問號？', '驚嘆!', '分號;'])
  })
  it('無句末標點 → 整段一行（不截斷）', () => {
    expect(splitFinalLines('這是很長的一段沒有句末標點的字')).toEqual(['這是很長的一段沒有句末標點的字'])
  })
  it('逗號/頓號不斷', () => {
    expect(splitFinalLines('甲，乙、丙。')).toEqual(['甲，乙、丙。'])
  })
  it('trim + 去空', () => {
    expect(splitFinalLines('  甲。  乙。 ')).toEqual(['甲。', '乙。'])
    expect(splitFinalLines('')).toEqual([])
  })
})
