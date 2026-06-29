import { init, command, observeProperties, getProperty, setProperty, destroy, listenEvents } from 'tauri-plugin-libmpv-api'
import type { MpvObservableProperty, MpvEvent } from 'tauri-plugin-libmpv-api'
import { resolveResource } from '@tauri-apps/api/path'

// initialOptions 依設定的影片輸出（vo）動態組（重啟生效，由 useSettings playback.videoOutput 帶入）。
// vo=gpu：HDR 穩定 tone-map 成 SDR、d3d11-flip=no 走 BitBlt 不透明呈現不變暗（嵌 WebView2 安全牌）。
// vo=gpu-next：畫質較佳、d3d11 預設 composition（v0.41 下不暗），但 HDR/libplacebo 偶發在某畫格凍住。
// 其餘：hwdec 硬解；字幕走 web overlay（關 mpv 自身 sid/sub-auto，否則疊字幕）；4K 網路預讀緩衝（dd04e35）；
// 強制可 seek + 緩存 seek（cold-seek 順）。seek 不限速由 mpv 內建 ytdl 重解 URL（loadViaYtdl）保證，與 vo 無關。
function buildInitialOptions(vo: 'gpu' | 'gpu-next'): Record<string, string> {
  const o: Record<string, string> = {
    vo,
    'gpu-context': 'd3d11',
    hwdec: 'auto-safe',
    'keep-open': 'yes',
    'force-window': 'yes',
    sid: 'no',
    'sub-auto': 'no',
    'demuxer-max-bytes': '256MiB',
    'demuxer-readahead-secs': '30',
    'force-seekable': 'yes',
    'demuxer-seekable-cache': 'yes',
  }
  if (vo === 'gpu') o['d3d11-flip'] = 'no' // 只有 vo=gpu 需 BitBlt 不透明呈現防嵌入變暗
  return o
}

// 觀察屬性：tuple [name, format] 或 [name, format, 'none']（值可為 null）
const OBSERVED = [
  ['pause', 'flag'],
  ['path', 'string', 'none'],
  ['time-pos', 'double', 'none'],
  ['duration', 'double', 'none'],
  ['volume', 'double'],
  ['mute', 'flag'],
  ['paused-for-cache', 'flag'],
  ['cache-buffering-state', 'double', 'none'],
  ['demuxer-cache-time', 'double', 'none'],
] as const satisfies readonly MpvObservableProperty[]

export interface MpvState {
  pause: boolean | null
  path: string | null
  timePos: number | null
  duration: number | null
  volume: number | null
  mute: boolean | null
  pausedForCache: boolean | null
  cacheBufferingState: number | null
  demuxerCacheTime: number | null
}

let last: MpvState = { pause: null, path: null, timePos: null, duration: null, volume: null, mute: null, pausedForCache: null, cacheBufferingState: null, demuxerCacheTime: null }

// observe callback 事件形狀為 { name, data }
function applyEvent(e: { name: string; data: unknown }): MpvState {
  if (e.name === 'pause') last = { ...last, pause: typeof e.data === 'boolean' ? e.data : null }
  else if (e.name === 'path') last = { ...last, path: typeof e.data === 'string' ? e.data : null }
  else if (e.name === 'time-pos') last = { ...last, timePos: typeof e.data === 'number' ? e.data : null }
  else if (e.name === 'duration') last = { ...last, duration: typeof e.data === 'number' ? e.data : null }
  else if (e.name === 'volume') last = { ...last, volume: typeof e.data === 'number' ? e.data : null }
  else if (e.name === 'mute') last = { ...last, mute: typeof e.data === 'boolean' ? e.data : null }
  else if (e.name === 'paused-for-cache') last = { ...last, pausedForCache: typeof e.data === 'boolean' ? e.data : null }
  else if (e.name === 'cache-buffering-state') last = { ...last, cacheBufferingState: typeof e.data === 'number' ? e.data : null }
  else if (e.name === 'demuxer-cache-time') last = { ...last, demuxerCacheTime: typeof e.data === 'number' ? e.data : null }
  return last
}

export async function startMpv(onState: (s: MpvState) => void, videoOutput: 'gpu' | 'gpu-next' = 'gpu'): Promise<void> {
  // 先掛 JS 監聽，再 init（init 才註冊 mpv_observe_property 並啟動 mpv），避免漏接初始事件
  await observeProperties(OBSERVED, (e) => { onState(applyEvent(e)) })
  await init({ initialOptions: buildInitialOptions(videoOutput), observedProperties: OBSERVED })
}

export async function loadFile(path: string): Promise<void> {
  await setProperty('audio-files', '') // 清掉前次遠端 DASH 的外部音軌，避免殘留套到本地檔
  await command('loadfile', [path])
  await setProperty('pause', false) // 解除 keep-open 在上一支 EOF 留下的暫停 → 新檔自動播放
}

// googlevideo 直連用乾淨桌面 UA：ffmpeg(lavf) backend 對 yt-dlp 那串額外 header 會回 HTTP 400；
// 只送乾淨 UA 則正常配速、可預讀、不被 400。URL 已自帶簽章，不需其它 header。
const DESKTOP_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0.0.0 Safari/537.36'

// 播放遠端直連 URL（muxed / 其他）：乾淨 UA、不送額外 header、清外部音軌。
export async function loadUrlWithHeaders(url: string, _headers: Record<string, string>): Promise<void> {
  await setProperty('user-agent', DESKTOP_UA)
  await setProperty('http-header-fields', '') // 不送額外 header（ffmpeg backend 會因之 400）
  await setProperty('audio-files', '') // muxed/其他遠端：用內嵌音訊，清掉外部音軌
  await command('loadfile', [url])
  await setProperty('pause', false) // 解除 keep-open 在上一支 EOF 留下的暫停 → 新來源自動播放
}

// 播放 DASH 影軌 + 外部 audio 軌（YouTube HD/4K：影音分離）。乾淨 UA + 不送額外 header（同上原因）。
export async function loadDashWithHeaders(videoUrl: string, audioUrl: string, _headers: Record<string, string>): Promise<void> {
  await setProperty('user-agent', DESKTOP_UA)
  await setProperty('http-header-fields', '')
  await setProperty('audio-files', audioUrl) // 外部音軌
  await command('loadfile', [videoUrl])
  await setProperty('pause', false)
}

// 播放 YouTube/遠端：交給 mpv 內建 ytdl_hook 重新解析 URL（不限速、可 cold seek）。
// 修「app 自己 yt-dlp -J 拿到的 googlevideo URL 被限速、seek 後 demuxer 抓不到資料卡死」——
// 同 itag 但 mpv ytdl 重解的 URL 不被限速（實測 ~40MB/s vs 自解 ~1.8MB/s）。fmt 例：
// '401+bestaudio/bestvideo+bestaudio/best'（pin 影軌 itag 控畫質、音軌交給 mpv）。yt-dlp 由 app
// 打包、經 PATH 找到（見 lib.rs setup 把 subs 目錄加進 PATH）。watchUrl 用 canonical YouTube 網址。
export async function loadViaYtdl(watchUrl: string, fmt: string): Promise<void> {
  await setProperty('ytdl-format', fmt)
  await setProperty('audio-files', '') // 不再用外部音軌；ytdl 自行合併影音
  await command('loadfile', [watchUrl])
  await setProperty('pause', false)
}

export async function togglePause(): Promise<void> {
  await command('cycle', ['pause'])
}

export async function seekAbsolute(sec: number): Promise<void> {
  await command('seek', [sec, 'absolute'])
}

export async function seekRelative(delta: number): Promise<void> {
  await command('seek', [delta, 'relative'])
}

export async function playlistNext(): Promise<void> {
  await command('playlist-next')
}

export async function playlistPrev(): Promise<void> {
  await command('playlist-prev')
}

export async function setVolume(vol: number): Promise<void> {
  await setProperty('volume', vol)
}

export async function setMute(on: boolean): Promise<void> {
  await setProperty('mute', on)
}

// 播放速度（mpv 內建 scaletempo 保音高）。
export async function setSpeed(v: number): Promise<void> { await setProperty('speed', v) }

// 影像調整：mpv 原生視訊均衡屬性，值域 -100..100。
export type ImageProp = 'brightness' | 'contrast' | 'saturation' | 'gamma' | 'hue'
export async function setImageProp(name: ImageProp, v: number): Promise<void> { await setProperty(name, v) }
export async function setDeband(on: boolean): Promise<void> { await setProperty('deband', on ? 'yes' : 'no') }

// 音訊濾鏡鏈（EQ + 正規化由 buildAf 統一組出；空字串＝清空）。
export async function setAf(af: string): Promise<void> { await setProperty('af', af) }

// 音訊延遲（A/V 同步）；mpv 單位為秒。
export async function setAudioDelay(sec: number): Promise<void> { await setProperty('audio-delay', sec) }

// 暫停狀態（畫質 reload 還原 pause 用；避免 usePlayer 直接 import setProperty）。
export async function setPause(on: boolean): Promise<void> { await setProperty('pause', on) }

// 設定 Modal 開啟時對影片套高斯模糊（磨砂玻璃背景）；關閉時移除。
// 走 gpu-next 的 GLSL user shader（GPU 算繪管線內，不破壞硬解、不吃 CPU）。
// shader 從 bundle resource 解析（dev 與安裝版皆可，不再硬寫原始碼路徑）。
// 路徑正規化成 mpv 吃的格式：去掉 Windows verbatim 前綴 \\?\、反斜線轉正斜線。
function toMpvPath(p: string): string {
  return p.replace(/^\\\\\?\\/, '').replace(/\\/g, '/')
}
let blurShaderPath: string | null = null
async function blurShader(): Promise<string> {
  if (blurShaderPath === null) blurShaderPath = toMpvPath(await resolveResource('shaders/blur.glsl'))
  return blurShaderPath
}
export async function setVideoBlur(on: boolean): Promise<void> {
  try {
    await setProperty('glsl-shaders', on ? await blurShader() : '')
  } catch (e) {
    console.error('[mpv] setVideoBlur failed', e) // 不再悄悄吞錯：失敗時明確報出
  }
}

// 浮動字幕模式：把 mpv 切成「只播音訊、放掉影像視窗」（→ Tauri 視窗透明），不 destroy。
//   hidden=true ：vid=no（停影像、音訊照播）+ force-window=no（無影像→釋放視窗）→ 透明
//   hidden=false：force-window=yes（先要回視窗）+ vid=auto（再開影像）→ 影像回來
// 好處：播放（音訊）連續、不需 destroy/restart、無事件雙送、無 auto-play 殘留 bug。
export async function setVideoHidden(hidden: boolean): Promise<void> {
  if (hidden) {
    await setProperty('vid', 'no')
    await setProperty('force-window', 'no')
  } else {
    await setProperty('force-window', 'yes')
    await setProperty('vid', 'auto')
  }
}

export async function currentHwdec(): Promise<string | null> {
  return await getProperty('hwdec-current', 'string')
}

// 解析 mpv 當前播放音軌的「實際 ffmpeg 串流索引」(ff-index)，給 ffmpeg `-map 0:<ff-index>` 用。
// 注意：mpv `aid` 是 1-based、ffmpeg `a:n` 是 0-based，故用 ff-index 避開差一。無音軌回 null。
export async function currentAudioFfIndex(): Promise<number | null> {
  const count = (await getProperty('track-list/count', 'int64')) as number | null
  if (!count) return null
  for (let i = 0; i < count; i++) {
    const type = await getProperty(`track-list/${i}/type`, 'string')
    const selected = await getProperty(`track-list/${i}/selected`, 'flag')
    if (type === 'audio' && selected) {
      return (await getProperty(`track-list/${i}/ff-index`, 'int64')) as number | null
    }
  }
  return null
}

export async function stopMpv(): Promise<void> {
  await destroy()
}

// 停止當前播放檔案（卸載、mpv 存活轉 idle）
export async function stop(): Promise<void> {
  await command('stop', [])
}

// 全 mpv 事件監聽（start-file / end-file / file-loaded ...）。usePlayer 用來驅動佇列自動下一支與最近記錄。
export function onMpvEvent(cb: (e: MpvEvent) => void): Promise<() => void> {
  return listenEvents(cb)
}
