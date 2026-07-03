import { describe, it, expect } from 'vitest'
import { decideTranslateEnable } from '../player/useTranslateProvision'

describe('decideTranslateEnable', () => {
  it('ask iff runtime(llmServer) missing; llmModel ignored', () => {
    expect(decideTranslateEnable([{ kind: 'llmServer' }])).toBe('ask')
    expect(decideTranslateEnable([{ kind: 'llmModel' }])).toBe('enable') // 模型不 gate 啟用
    expect(decideTranslateEnable([])).toBe('enable')
    expect(decideTranslateEnable([{ kind: 'llmServer' }, { kind: 'llmModel' }])).toBe('ask')
  })
})
