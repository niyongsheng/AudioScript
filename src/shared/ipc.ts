import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { t } from './i18n'
import { withTimeout } from './timeout'
import type {
  AudioStatus,
  EndpointConfig,
  InputDevice,
  MicRequestResult,
  ModelJobStatus,
  ModelStatus,
  ModelsList,
  OverlayConfig,
  PermissionSnapshot,
  Preferences,
  PrefsPatch,
  SessionInfo,
  SystemAudioProbe,
  TaskStatus,
  TranscriptSegment,
} from './types'
import {
  AudioSourceKind,
  Language,
  MicPermission,
  ModelPhase,
  type OverlayMode,
  SystemAudioVerdict,
  Theme,
  TranslateEngine,
  ZhScript,
} from './types'

export type {
  AudioStatus,
  InputDevice,
  ModelJobStatus,
  ModelStatus,
  ModelsList,
  OverlayConfig,
  PermissionSnapshot,
  Preferences,
  PrefsPatch,
  SystemAudioProbe,
}
export { AudioSourceKind, Language, MicPermission, ModelPhase, SystemAudioVerdict, Theme, TranslateEngine, ZhScript }

export async function call<T = unknown>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  return invoke<T>('dispatch', { name, args })
}

export const EVENT = {
  segments: 'transcript://segments',
  taskState: 'transcript://state',
  overlayState: 'overlay://state',
  overlayVisibility: 'overlay://visibility',
  audio: 'transcript://audio',
  level: 'transcript://level',
  languageMismatch: 'transcript://language-mismatch',
  deviceLevel: 'transcript://device-level',
  modelsProgress: 'models://progress',
  updateProgress: 'update://progress',
  locale: 'app://locale',
} as const

export function on<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (e) => handler(e.payload))
}

export interface AppInfo {
  name: string
  version: string
  platform: string
  arch: string
  builtAtMs: number | null
}

export interface AutostartState {
  enabled: boolean
}

export interface ConfigDefaults {
  overlay: OverlayConfig
  endpoint: EndpointConfig
  endpointProfiles: Record<'interpretation' | 'dictation', EndpointConfig>
}

export function appInfo(): Promise<AppInfo> {
  return call<AppInfo>('app_info')
}

export function localeGet(): Promise<string> {
  return call<string>('locale_get')
}

export interface UpdateCheckResult {
  currentVersion: string
  available: boolean
  version: string | null
  notes: string | null
}

export interface UpdateProgress {
  downloaded: number
  total: number | null
}

export function updateCheck(): Promise<UpdateCheckResult> {
  return call<UpdateCheckResult>('update_check')
}

export function updateInstall(): Promise<{ version: string }> {
  return call<{ version: string }>('update_install')
}

export function updateRestart(): Promise<null> {
  return call<null>('update_restart')
}

export interface SessionDevices {
  device?: string
  mixMicDeviceId?: string
  mixSystemDeviceId?: string
}

export function sessionStart(
  devices: SessionDevices = {},
  translate: TranslateEngine = TranslateEngine.LocalNmt,
  sourceLang = 'en',
  targetLang = 'zh',
  zhScript: ZhScript = ZhScript.Simplified,
  source: AudioSourceKind = AudioSourceKind.Loopback,
): Promise<TaskStatus> {
  return call<TaskStatus>('session_start', {
    device: devices.device || null,
    mixMicDeviceId: devices.mixMicDeviceId || null,
    mixSystemDeviceId: devices.mixSystemDeviceId || null,
    source,
    config: { translate: { engine: translate, sourceLang, targetLang, zhScript } },
  })
}

export function audioDevices(source: AudioSourceKind = AudioSourceKind.Loopback): Promise<InputDevice[]> {
  return call<InputDevice[]>('audio_devices', { source })
}

export interface DeviceLevelEvent {
  source: AudioSourceKind
  device: string | null
  rms: number
}

export interface MonitorTarget {
  source: AudioSourceKind
  device: string | null
}

export function monitorStart(targets: MonitorTarget[]): Promise<null> {
  return call<null>('monitor_start', {
    targets: targets.map((target) => ({ source: target.source, device: target.device || null })),
  })
}

export function monitorStop(): Promise<null> {
  return call<null>('monitor_stop')
}

export function prefsGet(): Promise<Preferences> {
  return call<Preferences>('prefs_get')
}

export function prefsSet(patch: PrefsPatch): Promise<Preferences> {
  return call<Preferences>('prefs_set', { patch })
}

export function prefsReset(): Promise<Preferences> {
  return call<Preferences>('prefs_reset')
}

export function configDefaults(): Promise<ConfigDefaults> {
  return call<ConfigDefaults>('config_defaults')
}

const MODELS_TIMEOUT_MS = 10_000

export function modelsList(): Promise<ModelsList> {
  return withTimeout(call<ModelsList>('models_list'), MODELS_TIMEOUT_MS, t('error.modelsTimeout'))
}

export function modelDownload(modelId: string): Promise<{ jobId: string }> {
  return call<{ jobId: string }>('model_download', { modelId })
}

export function modelOpenDir(): Promise<null> {
  return call<null>('model_open_dir')
}

export function modelCancel(jobId: string): Promise<{ cancelled: boolean }> {
  return call<{ cancelled: boolean }>('model_cancel', { jobId })
}

export function modelRemove(modelId: string): Promise<ModelsList> {
  return call<ModelsList>('model_remove', { modelId })
}

export function modelSelect(modelId: string): Promise<ModelsList> {
  return call<ModelsList>('model_select', { modelId })
}

export function autostartGet(): Promise<AutostartState> {
  return call<AutostartState>('autostart_get')
}

export function autostartSet(enabled: boolean): Promise<AutostartState> {
  return call<AutostartState>('autostart_set', { enabled })
}

export interface HotkeyInfo {
  id: string
  accelerator: string
  acceleratorPretty: string
  defaultAccelerator: string
  customized: boolean
  registered: boolean
}

export function shortcutsGet(): Promise<HotkeyInfo[]> {
  return call<HotkeyInfo[]>('shortcuts_get')
}

export function shortcutsSet(id: string, accelerator: string): Promise<HotkeyInfo[]> {
  return call<HotkeyInfo[]>('shortcuts_set', { id, accelerator })
}

export type PrivacyPane = 'microphone' | 'systemAudio'

export function openPrivacySettings(pane: PrivacyPane = 'systemAudio'): Promise<null> {
  return call<null>('open_privacy_settings', { pane })
}

export type PermissionKind = PrivacyPane

export interface CmdPermissionError {
  kind: 'permission'
  permission: PermissionKind
  message: string
}

export function permissionOf(e: unknown): CmdPermissionError | null {
  if (typeof e !== 'object' || e === null) return null
  const c = e as Partial<CmdPermissionError>
  return c.kind === 'permission' && typeof c.message === 'string' ? (c as CmdPermissionError) : null
}

export function formatCmdError(e: unknown): string {
  return permissionOf(e)?.message ?? String(e)
}

export function permissionStatus(): Promise<PermissionSnapshot> {
  return call<PermissionSnapshot>('permission_status')
}

export function permissionRequestMic(): Promise<MicRequestResult> {
  return call<MicRequestResult>('permission_request_mic')
}

export function permissionProbeSystemAudio(): Promise<SystemAudioProbe> {
  return call<SystemAudioProbe>('permission_probe_system_audio')
}

export function openRepo(): Promise<null> {
  return call<null>('open_repo')
}

export function openSponsor(): Promise<null> {
  return call<null>('open_sponsor')
}

export function openUrl(url: string): Promise<null> {
  return call<null>('open_url', { url })
}

export function revealPath(path?: string): Promise<null> {
  return call<null>('reveal_path', path ? { path } : {})
}

export function logsExport(): Promise<{ path: string }> {
  return call<{ path: string }>('logs_export')
}

export function transcriptExport(stamp: string, path: string): Promise<{ path: string }> {
  return call<{ path: string }>('transcript_export', { stamp, path })
}

export function overlayToggle(): Promise<boolean> {
  return call<boolean>('overlay_toggle')
}

export function overlaySetMode(mode: OverlayMode): Promise<OverlayMode> {
  return call<OverlayMode>('overlay_set_mode', { mode })
}

export function overlayConfigGet(): Promise<OverlayConfig> {
  return call<OverlayConfig>('overlay_config_get')
}

export function overlayConfigSet(config: OverlayConfig): Promise<OverlayConfig> {
  return call<OverlayConfig>('overlay_config_set', { config })
}

export function overlayIsVisible(): Promise<boolean> {
  return call<boolean>('overlay_visible')
}

export function sessionStop(): Promise<TaskStatus> {
  return call<TaskStatus>('session_stop')
}

export function sessionSnapshot(): Promise<TranscriptSegment[]> {
  return call<TranscriptSegment[]>('session_snapshot')
}

export function sessionStatus(): Promise<SessionInfo> {
  return call<SessionInfo>('session_status')
}

export function sessionClear(): Promise<void> {
  return call<void>('session_clear')
}

export interface HotkeyFailure {
  id: string
  accelerator: string
}

export function hotkeyFailures(): Promise<HotkeyFailure[]> {
  return call<HotkeyFailure[]>('hotkey_failures')
}
