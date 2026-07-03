import { describe, it, expect } from 'vitest'
import { transHintText } from '../player/modelRows'

describe('transHintText', () => {
  it('3-state hint', () => {
    expect(transHintText('off', true, true)).toBe('需先選字幕來源')
    expect(transHintText('live', false, false)).toBe('需在設定啟用翻譯')
    expect(transHintText('live', true, false)).toBe('需在設定下載翻譯模型')
    expect(transHintText('live', true, true)).toBe('') // 可翻、無提示
  })
})
