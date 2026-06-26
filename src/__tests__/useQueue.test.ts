import { describe, it, expect, vi, beforeEach } from 'vitest'

// 捕捉 ensureWired 註冊的事件 callback 到模組變數：第一次 useQueue() 時設定一次（once-guard），
// 不受 vi.clearAllMocks() 影響 → 供事件守衛測試取用。
let endCb: ((reason: string, entryId: number) => void) | undefined
let fileLoadedCb: (() => void) | undefined
let startFileCb: ((entryId: number) => void) | undefined

const player = {
  loadPath: vi.fn(async (_id: string) => true),
  playResolvedRemote: vi.fn(async (_id: string, _r: unknown) => true),
  setResolving: vi.fn(),
  notify: vi.fn(),
  onEndFile: vi.fn((cb) => { endCb = cb }),
  onFileLoaded: vi.fn((cb) => { fileLoadedCb = cb }),
  onStartFile: vi.fn((cb) => { startFileCb = cb }),
}
const recent = { record: vi.fn(), load: vi.fn(), refreshMissing: vi.fn(), items: [] }
vi.mock('../player/usePlayer', () => ({ usePlayer: () => player }))
vi.mock('../player/useRecent', () => ({ useRecent: () => recent }))
vi.mock('../player/backend', () => ({
  resolveRemote: vi.fn(async () => ({ playbackUrl: 'u', httpHeaders: {}, isLive: false })),
  stopExternalTranscription: vi.fn(async () => {}),
}))

import { useQueue } from '../player/useQueue'
import { resolveRemote } from '../player/backend'

const L = (id: string) => ({ kind: 'local' as const, id, title: id })
const R = (id: string) => ({ kind: 'remote' as const, id, title: id })

beforeEach(() => {
  vi.clearAllMocks()
  // clearAllMocks 只清呼叫歷史、不清 mockImplementation → 重設 resolveRemote 預設，避免競態測試的 deferred impl 外洩。
  ;(resolveRemote as any).mockImplementation(async () => ({ playbackUrl: 'u', httpHeaders: {}, isLive: false }))
  useQueue().clear()
})

describe('enqueueItems', () => {
  it('空佇列 → 播第一支', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b')])
    expect(q.items.length).toBe(2)
    expect(q.state.index).toBe(0)
    expect(player.loadPath).toHaveBeenCalledWith('a')
  })
  it('非空 + 無 opts → append 不打斷', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')])               // 播 a
    player.loadPath.mockClear()
    await q.enqueueItems([L('b')])               // append，不應再 load
    expect(q.items.length).toBe(2)
    expect(player.loadPath).not.toHaveBeenCalled()
  })
  it('清單空佇列 → 從 startOffset 播', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')], { startOffset: 1 })
    expect(player.loadPath).toHaveBeenCalledWith('b')
  })
  it('interrupt 非空 → append 並跳到新項', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')])
    player.loadPath.mockClear()
    await q.enqueueItems([L('z')], { interrupt: true })
    expect(player.loadPath).toHaveBeenCalledWith('z')
  })
  it('非空 + startOffset（清單 append）→ 不跳（Q1）', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')])               // 播 a
    player.loadPath.mockClear()
    await q.enqueueItems([L('p'), L('q'), L('r')], { startOffset: 1 }) // 清單 append 到尾
    expect(q.items.map((x) => x.id)).toEqual(['a', 'p', 'q', 'r'])
    expect(q.state.index).toBe(0)               // 仍在 a，不打斷
    expect(player.loadPath).not.toHaveBeenCalled()
  })
  it('空佇列 + interrupt → 直接播', async () => {
    const q = useQueue()
    await q.enqueueItems([L('only')], { interrupt: true })
    expect(player.loadPath).toHaveBeenCalledWith('only')
    expect(q.state.index).toBe(0)
  })
  it('noAutoplay suppresses autoplay on empty queue', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')], { noAutoplay: true })
    expect(q.items.length).toBe(1)
    expect(player.loadPath).not.toHaveBeenCalled()
  })
})

describe('next / remove / move', () => {
  it('next 超尾停', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b')])
    await q.next()                               // → b
    expect(q.state.index).toBe(1)
    player.loadPath.mockClear()
    await q.next()                               // 超尾 → no-op
    expect(q.state.index).toBe(1)
    expect(player.loadPath).not.toHaveBeenCalled()
  })
  it('remove 非 current → 修正 index', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])
    await q.playAt(2)                            // current=c (index 2)
    q.remove(0)                                  // 移除 a → c 變 index 1
    expect(q.state.index).toBe(1)
    expect(q.items.map((x) => x.id)).toEqual(['b', 'c'])
  })
  it('remove current → 遞補同位置', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])
    await q.playAt(1)                            // current=b
    player.loadPath.mockClear()
    q.remove(1)                                  // 移除 b → c 補到 index 1 並播
    expect(q.items.map((x) => x.id)).toEqual(['a', 'c'])
    expect(player.loadPath).toHaveBeenCalledWith('c')
  })
  it('remove 在 current 之後 → index 不變、不重播', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])
    await q.playAt(0)                            // current=a
    player.loadPath.mockClear()
    q.remove(2)                                  // 移除 c（在 current 之後）
    expect(q.state.index).toBe(0)
    expect(player.loadPath).not.toHaveBeenCalled()
  })
  it('prev 回上一支；index 0 時 no-op', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b')])
    await q.next()                              // → b (index 1)
    player.loadPath.mockClear()
    await q.prev()                              // → a
    expect(q.state.index).toBe(0)
    expect(player.loadPath).toHaveBeenCalledWith('a')
    player.loadPath.mockClear()
    await q.prev()                              // index 0 → no-op
    expect(player.loadPath).not.toHaveBeenCalled()
  })
  it('move 把後面的項移到 current 前 → index++（指向同一邏輯項）', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])
    await q.playAt(1)                           // current=b (index 1)
    q.move(2, 0)                                // c 移到最前：[c,a,b] → b 變 index 2
    expect(q.items.map((x) => x.id)).toEqual(['c', 'a', 'b'])
    expect(q.state.index).toBe(2)
  })
  it('move 修正 index 指向同一邏輯項', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])  // current=a (index 0)
    q.move(0, 2)                                 // a 移到尾 → current 仍是 a，index 2
    expect(q.items.map((x) => x.id)).toEqual(['b', 'c', 'a'])
    expect(q.state.index).toBe(2)
  })
})

describe('playAt 競態（MAJOR-1）', () => {
  it('後發 playAt 永遠贏 source/index（不論 resolve 完成順序）', async () => {
    const q = useQueue()
    let resolve2: (() => void) | undefined
    let resolve5: (() => void) | undefined
    ;(resolveRemote as any).mockImplementation((id: string) =>
      id === 'r2'
        ? new Promise((r) => (resolve2 = () => r({ playbackUrl: 'u2', httpHeaders: {}, isLive: false })))
        : new Promise((r) => (resolve5 = () => r({ playbackUrl: 'u5', httpHeaders: {}, isLive: false }))))
    q.items.push(R('r0'), R('r1'), R('r2'), R('r3'), R('r4'), R('r5'))
    const p2 = q.playAt(2)
    const p5 = q.playAt(5)
    resolve5!(); resolve2!()
    await Promise.all([p2, p5])
    expect(q.state.index).toBe(5)
    const calls = player.playResolvedRemote.mock.calls.map((c) => c[0])
    expect(calls).toContain('r5')
    expect(calls).not.toContain('r2')
  })

  it('反向完成順序（r2 先、r5 後）也是後發 playAt(5) 贏', async () => {
    const q = useQueue()
    let resolve2: (() => void) | undefined
    let resolve5: (() => void) | undefined
    ;(resolveRemote as any).mockImplementation((id: string) =>
      id === 'r2'
        ? new Promise((r) => (resolve2 = () => r({ playbackUrl: 'u2', httpHeaders: {}, isLive: false })))
        : new Promise((r) => (resolve5 = () => r({ playbackUrl: 'u5', httpHeaders: {}, isLive: false }))))
    q.items.push(R('r0'), R('r1'), R('r2'), R('r3'), R('r4'), R('r5'))
    const p2 = q.playAt(2)
    const p5 = q.playAt(5)
    resolve2!(); resolve5!()                      // 反向：r2 先完成
    await Promise.all([p2, p5])
    expect(q.state.index).toBe(5)
    const calls = player.playResolvedRemote.mock.calls.map((c) => c[0])
    expect(calls).toContain('r5')
    expect(calls).not.toContain('r2')
  })
})

describe('onEndFile / onFileLoaded 守衛（事件無 identity → liveGen + entryId 關聯）', () => {
  it('eof + entryId 相符 + playGen===liveGen → next()', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b')])   // 播 a；liveGen 設定、expectStartFile=true
    startFileCb!(1)                           // start-file → liveEntryId=1
    player.loadPath.mockClear()
    endCb!('eof', 1)
    await Promise.resolve()
    expect(player.loadPath).toHaveBeenCalledWith('b')
  })

  it('entryId 不符（屬別的檔）→ 不 next', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b')])
    startFileCb!(1)
    player.loadPath.mockClear()
    endCb!('eof', 999)
    await Promise.resolve()
    expect(player.loadPath).not.toHaveBeenCalled()
  })

  it('有更新的 playAt 在飛（playGen≠liveGen）→ 不 next', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), R('r')])   // 播 a（local）；liveGen 設定
    startFileCb!(1)
    ;(resolveRemote as any).mockImplementationOnce(() => new Promise(() => {})) // 卡住的 resolve
    void q.playAt(2)                                  // remote in-flight：playGen++、liveGen 不變
    player.loadPath.mockClear()
    endCb!('eof', 1)                                  // 屬舊檔 a → 被 playGen≠liveGen 擋下
    await Promise.resolve()
    expect(player.loadPath).not.toHaveBeenCalled()
  })

  it('file-loaded → record liveItem（非 items[index]）', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')])                    // liveItem=a
    fileLoadedCb!()
    expect(recent.record).toHaveBeenCalledWith(expect.objectContaining({ id: 'a' }))
  })

  it('佇列播完(eof 到尾)後 append 新項 → 自動接續播（ended 狀態）', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a')])   // 播 a；index 0
    startFileCb!(1)
    player.loadPath.mockClear()
    endCb!('eof', 1)                 // a 播完、無下一支 → ended=true、停住
    await Promise.resolve()
    expect(player.loadPath).not.toHaveBeenCalled()
    await q.enqueueItems([L('b')])   // 拖入新項：非空但 ended → 自動播
    expect(player.loadPath).toHaveBeenCalledWith('b')
    expect(q.state.index).toBe(1)
  })

  it('commit 重置 liveEntryId → 漏捕 start-file 仍能 eof 前進（MAJOR 回歸）', async () => {
    const q = useQueue()
    await q.enqueueItems([L('a'), L('b'), L('c')])    // 播 a；commit liveEntryId=null
    startFileCb!(10)                                  // a 的 start-file → liveEntryId=10
    await q.next()                                    // → b：commit 重置 liveEntryId=null
    // 故意不發 b 的 start-file（模擬 start-file 與 loadfile-ack 亂序漏捕）
    player.loadPath.mockClear()
    endCb!('eof', 77)                                 // entryId 77 ≠ 舊的 10；但 liveEntryId 已 null → 不誤拒
    await Promise.resolve()
    expect(player.loadPath).toHaveBeenCalledWith('c') // 退回 playGen-only → 正確前進
  })
})
