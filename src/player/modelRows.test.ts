import { describe, it, expect } from 'vitest'
import { rowState, nextAutoSelect } from './modelRows'

describe('rowState', () => {
  const S = (...xs: string[]) => new Set(xs)
  it('downloading 優先於一切', () => {
    expect(rowState('turbo', 'turbo', S('turbo'), S('turbo'), S('turbo'))).toBe('downloading')
  })
  it('error 次於 downloading、優先於 idle', () => {
    expect(rowState('small', 'turbo', S(), S(), S('small'))).toBe('error')
  })
  it('已下載且為選取中 → active', () => {
    expect(rowState('turbo', 'turbo', S('turbo'), S(), S())).toBe('active')
  })
  it('已下載但非選取中 → downloaded', () => {
    expect(rowState('small', 'turbo', S('small'), S(), S())).toBe('downloaded')
  })
  it('未下載、未在下載、無錯 → idle', () => {
    expect(rowState('large-v3', 'turbo', S(), S(), S())).toBe('idle')
  })
})

describe('nextAutoSelect', () => {
  const S = (...xs: string[]) => new Set(xs)
  it('轉寫中一律 null（鎖死 MAJOR-A）', () => {
    expect(nextAutoSelect('turbo', S('small'), 'small', true)).toBeNull()
  })
  it('未轉寫、原選取已下載 → null（不亂切）', () => {
    expect(nextAutoSelect('turbo', S('turbo', 'small'), 'small', false)).toBeNull()
  })
  it('未轉寫、原選取未下載 → 選剛下載的那顆', () => {
    expect(nextAutoSelect('turbo', S('small'), 'small', false)).toBe('small')
  })
})
