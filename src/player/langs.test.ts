import { describe, it, expect } from 'vitest'
import { langToWhisper, LANGS } from './langs'

describe('langToWhisper', () => {
  it('auto → null/null', () => { expect(langToWhisper('auto')).toEqual({ lang: null, prompt: null }) })
  it('zh-Hant → zh + trad prompt', () => {
    const r = langToWhisper('zh-Hant'); expect(r.lang).toBe('zh'); expect(r.prompt).toContain('繁體')
  })
  it('zh-Hans → zh + simp prompt', () => {
    const r = langToWhisper('zh-Hans'); expect(r.lang).toBe('zh'); expect(r.prompt).toContain('简体')
  })
  it('ja → ja/null', () => { expect(langToWhisper('ja')).toEqual({ lang: 'ja', prompt: null }) })
  it('unknown → null/null (treated as auto)', () => { expect(langToWhisper('xx')).toEqual({ lang: null, prompt: null }) })
})

describe('LANGS', () => {
  it('starts with common order auto,zh-Hant,zh-Hans,ja,en,ko', () => {
    expect(LANGS.slice(0, 6).map((l) => l.value)).toEqual(['auto', 'zh-Hant', 'zh-Hans', 'ja', 'en', 'ko'])
  })
  it('values are unique', () => {
    expect(new Set(LANGS.map((l) => l.value)).size).toBe(LANGS.length)
  })
})
