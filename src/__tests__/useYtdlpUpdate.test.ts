import { describe, it, expect, vi, beforeEach } from 'vitest'

const h = vi.hoisted(() => ({
  settingsState: { capture: { enabled: true } },
  engineState: { open: false },
  translateState: { open: false },
  checkYtdlpUpdate: vi.fn(),
  updateYtdlp: vi.fn(),
  ready: { promise: Promise.resolve() as Promise<void> },
  listeners: [] as Array<(e: { payload: unknown }) => void>,
}))

vi.mock('../player/useSettings', () => ({
  useSettings: () => ({ state: h.settingsState }),
  whenSettingsReady: () => h.ready.promise,
}))
vi.mock('../player/backend', () => ({
  checkYtdlpUpdate: (...a: unknown[]) => h.checkYtdlpUpdate(...a),
  updateYtdlp: (...a: unknown[]) => h.updateYtdlp(...a),
}))
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_n: string, cb: (e: { payload: unknown }) => void) => { h.listeners.push(cb); return () => {} }),
}))
vi.mock('../player/useEngineProvision', () => ({ useEngineProvision: () => ({ state: h.engineState }) }))
vi.mock('../player/useTranslateProvision', () => ({ useTranslateProvision: () => ({ state: h.translateState }) }))

async function load() {
  vi.resetModules()
  const m = await import('../player/useYtdlpUpdate')
  return m.useYtdlpUpdate()
}

beforeEach(() => {
  h.settingsState.capture.enabled = true
  h.engineState.open = false
  h.translateState.open = false
  h.ready.promise = Promise.resolve()
  h.listeners.length = 0
  h.checkYtdlpUpdate.mockReset()
  h.updateYtdlp.mockReset()
  h.checkYtdlpUpdate.mockResolvedValue({ local: '2026.10.01', latest: '2026.10.26', outdated: true })
})

describe('useYtdlpUpdate.checkOnStartup', () => {
  it('capture disabled -> no check', async () => {
    h.settingsState.capture.enabled = false
    const y = await load()
    await y.checkOnStartup()
    expect(h.checkYtdlpUpdate).not.toHaveBeenCalled()
    expect(y.state.open).toBe(false)
  })
  it('waits for settings ready', async () => {
    let resolve!: () => void
    h.ready.promise = new Promise<void>((r) => { resolve = r })
    const y = await load()
    const p = y.checkOnStartup()
    await Promise.resolve()
    expect(h.checkYtdlpUpdate).not.toHaveBeenCalled()
    resolve()
    await p
    expect(h.checkYtdlpUpdate).toHaveBeenCalledTimes(1)
  })
  it('check rejects -> stays closed, no throw', async () => {
    h.checkYtdlpUpdate.mockRejectedValue('boom')
    const y = await load()
    await expect(y.checkOnStartup()).resolves.toBeUndefined()
    expect(y.state.open).toBe(false)
  })
  it('not outdated -> closed', async () => {
    h.checkYtdlpUpdate.mockResolvedValue({ local: 'a', latest: 'a', outdated: false })
    const y = await load()
    await y.checkOnStartup()
    expect(y.state.open).toBe(false)
  })
  it('engine dialog open -> closed', async () => {
    h.engineState.open = true
    const y = await load()
    await y.checkOnStartup()
    expect(y.state.open).toBe(false)
  })
  it('translate dialog open -> closed', async () => {
    h.translateState.open = true
    const y = await load()
    await y.checkOnStartup()
    expect(y.state.open).toBe(false)
  })
  it('outdated -> ask dialog', async () => {
    const y = await load()
    await y.checkOnStartup()
    expect(y.state.open).toBe(true)
    expect(y.state.phase).toBe('ask')
    expect(y.state.local).toBe('2026.10.01')
    expect(y.state.latest).toBe('2026.10.26')
  })
  it('later() closes', async () => {
    const y = await load()
    await y.checkOnStartup()
    y.later()
    expect(y.state.open).toBe(false)
  })
})

describe('useYtdlpUpdate.update', () => {
  it('success -> done with actual version', async () => {
    h.updateYtdlp.mockResolvedValue('2026.11.02')
    const y = await load()
    await y.update()
    expect(y.state.phase).toBe('done')
    expect(y.state.version).toBe('2026.11.02')
  })
  it('failure -> error, retry calls again', async () => {
    h.updateYtdlp.mockRejectedValueOnce('更新失敗：X').mockResolvedValueOnce('2026.11.02')
    const y = await load()
    await y.update()
    expect(y.state.phase).toBe('error')
    expect(y.state.error).toContain('更新失敗：X')
    await y.update()
    expect(h.updateYtdlp).toHaveBeenCalledTimes(2)
    expect(y.state.phase).toBe('done')
  })
  it('double call while downloading -> once', async () => {
    let resolve!: (v: string) => void
    h.updateYtdlp.mockReturnValue(new Promise<string>((r) => { resolve = r }))
    const y = await load()
    const p1 = y.update()
    const p2 = y.update()
    await Promise.resolve(); await Promise.resolve(); await Promise.resolve()
    resolve('1')
    await Promise.all([p1, p2])
    expect(h.updateYtdlp).toHaveBeenCalledTimes(1)
  })
  it('close() ignored while downloading', async () => {
    let resolve!: (v: string) => void
    h.updateYtdlp.mockReturnValue(new Promise<string>((r) => { resolve = r }))
    const y = await load()
    y.state.open = true
    const p = y.update()
    y.close()
    expect(y.state.open).toBe(true)
    resolve('1')
    await p
    y.close()
    expect(y.state.open).toBe(false)
  })
  it('progress events update pct/MB, other phases ignored', async () => {
    let resolve!: (v: string) => void
    h.updateYtdlp.mockReturnValue(new Promise<string>((r) => { resolve = r }))
    const y = await load()
    const p = y.update()
    for (let i = 0; i < 5; i++) await Promise.resolve()
    h.listeners[0]({ payload: { phase: 'ytdlp', done: 1, total: 2, message: '' } })
    expect(y.state.pct).toBe(0)
    h.listeners[0]({ payload: { phase: 'ytdlp-update', done: 5_000_000, total: 10_000_000, message: '' } })
    expect(y.state.pct).toBe(50)
    expect(y.state.doneMb).toBe(5)
    expect(y.state.totalMb).toBe(10)
    resolve('1')
    await p
  })
})
