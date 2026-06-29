import { describe, it, expect, vi, beforeEach } from 'vitest'

// vi.mock is hoisted to the top of the file by vitest; factory functions cannot reference
// const variables declared below. Use vi.hoisted() to declare shared mocks safely.
const {
  settingsState,
  subsTracks,
  notify,
  setVideoHidden,
} = vi.hoisted(() => {
  const settingsState = { liveSubs: { enabled: true, sourceLang: 'ja', display: { lines: 3 } }, appearance: { primary: { fontSize: 28 } }, floating: { x: null, y: null, width: null } }
  const subsTracks = { primary: { source: 'off', delaySec: 0 }, secondary: { source: 'off', delaySec: 0 } }
  return {
    settingsState,
    subsTracks,
    notify: vi.fn(),
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
vi.mock('../player/useSubtitles', () => ({ useSubtitles: () => ({ tracks: subsTracks }) }))
vi.mock('../mpv', () => ({ setVideoHidden }))

import { useFloatingMode } from '../player/useFloatingMode'

beforeEach(() => {
  vi.clearAllMocks()
  subsTracks.primary.source = 'off'
  const f = useFloatingMode(); if (f.active.value) f._forceReset()
})

describe('useFloatingMode guards', () => {
  it('enters successfully: hides video (audio-only)', async () => {
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(true)
    expect(setVideoHidden).toHaveBeenCalledWith(true)
  })
  it('re-entry guard: second enter while active is a no-op', async () => {
    const f = useFloatingMode(); await f.enter(); await f.enter()
    expect(setVideoHidden).toHaveBeenCalledTimes(1)
  })
  it('exit restores video', async () => {
    const f = useFloatingMode(); await f.enter()
    setVideoHidden.mockClear()
    await f.exit()
    expect(f.active.value).toBe(false)
    expect(setVideoHidden).toHaveBeenCalledWith(false)
  })
  it('exit is idempotent when not active', async () => {
    const f = useFloatingMode(); await f.exit()
    expect(setVideoHidden).not.toHaveBeenCalled()
  })
  it('enters regardless of engine/lang settings (no guards)', async () => {
    settingsState.liveSubs.enabled = false    // 引擎未啟用不擋
    settingsState.liveSubs.sourceLang = 'auto'
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(true)
    expect(setVideoHidden).toHaveBeenCalledWith(true)
    expect(notify).not.toHaveBeenCalled()
  })
  it('enters with existing subs (source=live): no guards, hides video', async () => {
    subsTracks.primary.source = 'live'
    settingsState.liveSubs.enabled = false    // 引擎未啟用也不擋（沿用不需引擎）
    settingsState.liveSubs.sourceLang = 'auto'
    const f = useFloatingMode(); await f.enter()
    expect(f.active.value).toBe(true)
    expect(setVideoHidden).toHaveBeenCalledWith(true)
    expect(notify).not.toHaveBeenCalled()
  })
})
