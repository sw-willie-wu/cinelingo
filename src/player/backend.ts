import { invoke } from '@tauri-apps/api/core'

export interface HwInfo { backend: 'cuda' | 'vulkan' | 'cpu'; gpuName: string | null; hasGpu: boolean }
export interface ModelStatus { key: string; downloaded: boolean }

export const detectHardware = () => invoke<HwInfo>('detect_hardware')
export const listModels = () => invoke<ModelStatus[]>('list_models')
export const loadSettingsRaw = () => invoke<unknown>('load_settings')
export const saveSettingsRaw = (data: unknown) => invoke<void>('save_settings', { data })
export const downloadModel = (key: string) => invoke<void>('download_model', { key })

export interface MissingAsset { kind: string; sizeMb: number }
export interface EngineStatus { backendKind: string; missing: MissingAsset[] }

export const checkEngine = () => invoke<EngineStatus>('check_engine')
export const provisionEngine = () => invoke<void>('provision_engine')
export const readTextFile = (path: string) => invoke<string>('read_text_file', { path })
export interface SidecarSub { name: string; path: string }
export const listSidecarSubs = (videoPath: string) =>
  invoke<SidecarSub[]>('list_sidecar_subs', { videoPath })
export const loadSubMemory = () => invoke<Record<string, unknown>>('load_sub_memory')
export const saveSubMemory = (data: unknown) => invoke<void>('save_sub_memory', { data })
export const loadPlaybackMemory = () => invoke<Record<string, unknown>>('load_playback_memory')
export const savePlaybackMemory = (data: unknown) => invoke<void>('save_playback_memory', { data })

export interface CcTrack { lang: string; label: string; auto: boolean; vttUrl: string }
export interface VideoFormat { itag: string; height: number; fps: number; codec: string; tbr: number; url: string }
export interface ResolvedRemote {
  playbackUrl: string | null
  audioUrl: string | null
  httpHeaders: Record<string, string>
  durationSec: number
  isLive: boolean
  ccTracks: CcTrack[]
  title: string | null
  videos: VideoFormat[]
}
export const checkYtdlp = () => invoke<boolean>('check_ytdlp')
export const provisionYtdlp = () => invoke<void>('provision_ytdlp')
export const resolveRemote = (url: string) => invoke<ResolvedRemote>('resolve_remote', { url })

export interface FlatEntry { id: string; title: string }
export interface FlatPlaylist { title: string | null; entries: FlatEntry[] }
export const enumeratePlaylist = (url: string) => invoke<FlatPlaylist>('enumerate_playlist', { url })
export const remoteTitle = (url: string) => invoke<string | null>('remote_title', { url })

export const loadRecent = () => invoke<unknown>('load_recent')
export const saveRecent = (data: unknown) => invoke<void>('save_recent', { data })
export const expandPlayablePaths = (paths: string[]) => invoke<string[]>('expand_playable_paths', { paths })
export const pathExists = (path: string) => invoke<boolean>('path_exists', { path })

export interface ProcessSource { pid: number; name: string }
export interface InputDevice { id: string; name: string; isDefault: boolean }
export interface AudioSources { processes: ProcessSource[]; inputDevices: InputDevice[] }
export const listAudioSources = () => invoke<AudioSources>('list_audio_sources')
export const armAudioSource = (source: import('./useAudioSource').AudioSource) => invoke<void>('arm_audio_source', { source })
export const disarmAudioSource = () => invoke<void>('disarm_audio_source')
export const startExternalTranscription = (
  model: string,
  sourceLang: string,
  prompt: string,
  vadThreshold: number,
  vadMinSilenceMs: number,
) => invoke<void>('start_external_transcription', { model, sourceLang, prompt, vadThreshold, vadMinSilenceMs })
export const stopExternalTranscription = () => invoke<void>('stop_external_transcription')

export const startLoopback = (
  deviceId: string | null,
  model: string,
  sourceLang: string,
  prompt: string,
  vadThreshold: number,
  vadMinSilenceMs: number,
) =>
  invoke<void>('start_loopback_transcription', { deviceId, model, sourceLang, prompt, vadThreshold, vadMinSilenceMs })
export const stopLoopback = () => invoke<void>('stop_loopback_transcription')
