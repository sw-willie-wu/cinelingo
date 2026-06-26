import { describe, it, expect, vi, beforeEach } from 'vitest'

// vi.mock is hoisted to the top of the file by vitest; factory functions cannot reference
// const variables declared below. Use vi.hoisted() to declare shared mocks safely.
const {
  settingsState,
  subsTracks,
  notify,
  startLoopbackCapture,
  stopLoopbackCapture,
  setVideoHidden,
} = vi.hoisted(() => {
  const settingsState = { liveSubs: { enabled: true, sourceLang: 'ja', display: { lines: 3 } }, appearance: { primary: { fontSize: 28 } }, floating: { x: null, y: null, width: null } }
  const subsTracks = { primary: { source: 'off', delaySec: 0 }, secondary: { source: 'off', delaySec: 0 } }
  return {
    settingsState,
    subsTracks,
    notify: vi.fn(),
    startLoopbackCapture: vi.fn().mockResolvedValue(undefined),
    stopLoopbackCapture: vi.fn().mockResolvedValue(undefined),
    setVideoHidden: vi.fn().mockResolvedValue(undefined),
  }
})

vi.mock('@tauri-apps/api/window', () => ({ availableMonitors: vi.fn().mockResolvedValue([{ position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 } }]) }))
vi.mock('@tauri-apps/api/dpi', () => ({ PhysicalSize: class { constructor(public width: number, public height: number) {} }, PhysicalPosition: class { constructor(public x: number, public y: number) {} } }))
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({
    outerPosition: vi.fn().mockResolvedValue({ x: 0, y: 0 }),
    outerSize: vi.fn().mockResolvedValue({ width: 1000, height: 120 }),
    isFullscreen: vi.fn().mockResolvedValue(false),
    setFullscreen: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setAlwaysOnTop: vi.fn().mockResolvedValue(undefined),
    setSkipTaskbar: vi.fn().mockResolvedValue(undefined),
    setShadow: vi.fn().mockResolvedValue(undefined),
  }),
}))
vi.mock('../player/useSettings', () => ({ useSettings: () => ({ state: settingsState }) }))
vi.mock('../player/usePlayer', () => ({ usePlayer: () => ({ notify }) }))
vi.mock('../player/useSubtitles', () => ({ useSubtitles: () => ({ startLoopbackCapture, stopLoopbackCapture, tracks: subsTracks }) }))
vi.mock('../mpv', () => ({ setVideoHidden }))

import { useFloatingMode } from '../player/useFloatingMode'

beforeEach(() => {
  vi.clearAllMocks()
  settingsState.liveSubs.enabled = true
  settingsState.liveSubs.sourceLang = 'ja'
  subsTracks.primary.source = 'off'   // 預設無既有字幕 → 走 loopback
  const f = useFloatingMode(); if (f.active.value) f._forceReset()
})

describe('useFloatingMode guards', () => {
  it('blocks enter when engine not enabled', async () => {
    settingsState.liveSubs.enabled = false
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(false); expect(notify).toHaveBeenCalled(); expect(setVideoHidden).not.toHaveBeenCalled()
  })
  it('blocks enter when sourceLang is auto', async () => {
    settingsState.liveSubs.sourceLang = 'auto'
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(false); expect(notify).toHaveBeenCalled(); expect(setVideoHidden).not.toHaveBeenCalled()
  })
  it('enters successfully: hides video (audio-only) + starts loopback', async () => {
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(true)
    expect(setVideoHidden).toHaveBeenCalledWith(true)   // mpv 只播音訊、放掉影像視窗（不 destroy）
    expect(startLoopbackCapture).toHaveBeenCalledWith(null)
  })
  it('re-entry guard: second enter while active is a no-op', async () => {
    const f = useFloatingMode(); await f.enter(); await f.enter()
    expect(setVideoHidden).toHaveBeenCalledTimes(1)
  })
  it('rolls back (restores video) when loopback start fails', async () => {
    startLoopbackCapture.mockRejectedValueOnce(new Error('boom'))
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(false)
    expect(setVideoHidden).toHaveBeenCalledWith(false)   // rollback 還原影像
    expect(notify).toHaveBeenCalled()
  })
  it('exit restores video + stops loopback', async () => {
    const f = useFloatingMode(); await f.enter()
    setVideoHidden.mockClear()
    await f.exit()
    expect(f.active.value).toBe(false)
    expect(stopLoopbackCapture).toHaveBeenCalled()
    expect(setVideoHidden).toHaveBeenCalledWith(false)
  })
  it('exit is idempotent when not active', async () => {
    const f = useFloatingMode(); await f.exit()
    expect(stopLoopbackCapture).not.toHaveBeenCalled()
  })

  it('reuses existing clock subs: no loopback, no engine/lang guard', async () => {
    subsTracks.primary.source = 'live'        // mode A 即時字幕（或字幕檔）已啟用
    settingsState.liveSubs.enabled = false    // 引擎未啟用也不該擋（沿用不需引擎）
    settingsState.liveSubs.sourceLang = 'auto'
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(true)
    expect(setVideoHidden).toHaveBeenCalledWith(true)
    expect(startLoopbackCapture).not.toHaveBeenCalled()   // 沿用既有字幕 → 不開 loopback
    expect(notify).not.toHaveBeenCalled()                 // 守衛被跳過
  })

  it('reuse mode exit does not stop the existing subs (no loopback to stop)', async () => {
    subsTracks.primary.source = 'live'
    const f = useFloatingMode(); await f.enter()
    await f.exit()
    expect(f.active.value).toBe(false)
    expect(stopLoopbackCapture).not.toHaveBeenCalled()    // 沒開 loopback → 退出不停既有字幕
    expect(setVideoHidden).toHaveBeenCalledWith(false)    // 影像還是回來
  })
})
