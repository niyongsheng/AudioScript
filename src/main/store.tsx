import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import { setTrackingEnabled, setTrackingTier, track } from '@/shared/analytics'
import { t } from '@/shared/i18n'
import {
  type AppInfo,
  type AppRef,
  type AudioProcessInfo,
  AudioSourceKind,
  type AudioStatus,
  appInfo,
  audioDevices,
  type CmdPermissionError,
  EngineType,
  EVENT,
  audioProcesses as fetchAudioProcesses,
  formatCmdError,
  type HotkeyFailure,
  type HotkeyInfo,
  hotkeyFailures,
  type InputDevice,
  Language,
  type MacSpeechStatus,
  macSpeechStatus,
  type OverlayConfig,
  type OverlayConfigPatch,
  on,
  overlayConfigGet,
  overlayConfigPatch,
  overlayIsVisible,
  overlayToggle,
  type PermissionSnapshot,
  type Preferences,
  type PrefsPatch,
  permissionOf,
  permissionProbeSystemAudio,
  permissionStatus,
  prefsGet,
  prefsSet,
  type SystemAudioProbe,
  type SystemSettingsPane,
  sessionClear,
  sessionStart,
  sessionStatus,
  sessionStop,
  settingsOf,
  shortcutsGet,
  Theme,
  TranslateEngine,
  type UpdateCheckResult,
  type UpdateProgress,
  updateCheck,
  updateInstall,
  updateRestart,
  ZhScript,
} from '@/shared/ipc'
import { initLocale } from '@/shared/locale'
import { withTimeout } from '@/shared/timeout'
import { TaskStatus } from '@/shared/types'

type EndpointConfig = Preferences['endpoint']

export type UpdatePhase = 'idle' | 'checking' | 'downloading' | 'ready' | 'error'

interface Store {
  info: AppInfo | null

  prefsReady: boolean
  devicesLoading: boolean
  audioSource: AudioSourceKind
  device: string
  mixMicDevice: string
  mixSystemDevice: string
  appTargets: AppRef[]
  translateEngine: TranslateEngine
  asrEngine: EngineType
  sourceLang: string
  targetLang: string
  zhScript: ZhScript
  theme: Theme
  language: Language
  endpoint: EndpointConfig

  status: TaskStatus
  activeDevice: string | null
  audio: AudioStatus | null
  languageMismatch: boolean
  setLanguageMismatch: (v: boolean) => void

  devices: InputDevice[]
  mixMicDevices: InputDevice[]
  mixSystemDevices: InputDevice[]
  audioProcesses: AudioProcessInfo[]
  processesLoading: boolean
  processError: string | null
  deviceError: string | null

  overlayCfg: OverlayConfig | null
  overlayVisible: boolean

  hotkeys: HotkeyInfo[]
  hotkeyProblems: HotkeyFailure[]

  updateInfo: UpdateCheckResult | null
  updatePhase: UpdatePhase
  updateProgress: UpdateProgress | null
  updateError: string | null
  installedVersion: string | null
  checkUpdate: (silent?: boolean) => Promise<void>
  installUpdate: () => Promise<void>
  restartApp: () => Promise<void>

  onboardingDone: boolean
  completeOnboarding: (from: 'start' | 'later') => Promise<void>

  analyticsEnabled: boolean
  setAnalyticsEnabled: (v: boolean) => void

  permissions: PermissionSnapshot | null
  refreshPermission: () => Promise<void>
  speechStatus: MacSpeechStatus | null
  setSpeechStatus: (status: MacSpeechStatus | null) => void
  refreshSpeechStatus: () => Promise<void>
  systemAudioProbe: SystemAudioProbe | null
  runSystemAudioProbe: () => Promise<SystemAudioProbe>
  permissionError: CmdPermissionError | null
  setPermissionError: (e: CmdPermissionError | null) => void

  error: string | null
  errorSettingsPane: SystemSettingsPane | null
  setError: (e: string | null) => void
  pending: boolean

  toggleSession: () => Promise<void>
  clearAll: () => Promise<void>
  setAudioSource: (v: AudioSourceKind) => void
  setDevice: (v: string) => void
  setMixMicDevice: (v: string) => void
  setMixSystemDevice: (v: string) => void
  setAppTargets: (v: AppRef[]) => void
  setTranslateEngine: (v: TranslateEngine) => void
  setAsrEngine: (v: EngineType) => void
  setEngineAndDirection: (engine: TranslateEngine, source: string, target: string, script: ZhScript) => void
  setDirection: (source: string, target: string, script: ZhScript) => void
  changeTheme: (v: Theme) => Promise<void>
  changeLanguage: (v: Language) => Promise<void>
  setEndpoint: (v: EndpointConfig) => Promise<void>
  updateOverlay: (patch: OverlayConfigPatch) => Promise<void>
  toggleOverlay: () => Promise<void>
  refreshDevices: () => Promise<void>
  refreshProcesses: () => Promise<void>
  refreshHotkeys: () => Promise<void>
}

const STOP_TIMEOUT_MS = 8_000

const START_TIMEOUT_MS = 60_000

const PROCESS_REFRESH_MIN_MS = 500

const Ctx = createContext<Store | null>(null)

export function useStore(): Store {
  const s = useContext(Ctx)
  if (!s) throw new Error('useStore 必须在 <StoreProvider> 内使用')
  return s
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [prefsReady, setPrefsReady] = useState(false)

  const [audioSource, setAudioSource] = useState<AudioSourceKind>(AudioSourceKind.Loopback)
  const [device, setDevice] = useState('')
  const [mixMicDevice, setMixMicDevice] = useState('')
  const [mixSystemDevice, setMixSystemDevice] = useState('')
  const [appTargets, setAppTargetsState] = useState<AppRef[]>([])
  const [translateEngine, setTranslateEngine] = useState<TranslateEngine>(TranslateEngine.LocalNmt)
  const [asrEngine, setAsrEngine] = useState<EngineType>(EngineType.LocalStreaming)
  const [sourceLang, setSourceLang] = useState('en')
  const [targetLang, setTargetLang] = useState('zh')
  const [zhScript, setZhScript] = useState(ZhScript.Simplified)
  const [theme, setTheme] = useState<Theme>(Theme.System)
  const [language, setLanguage] = useState<Language>(Language.System)
  const [endpoint, setEndpoint] = useState<EndpointConfig>({
    trailingSilenceMs: 800,
    maxUtteranceMs: 8000,
  })

  const [status, setStatus] = useState<TaskStatus>(TaskStatus.Ready)
  const [activeDevice, setActiveDevice] = useState<string | null>(null)
  const [audio, setAudio] = useState<AudioStatus | null>(null)
  const [languageMismatch, setLanguageMismatch] = useState(false)

  const [devices, setDevices] = useState<InputDevice[]>([])
  const [mixMicDevices, setMixMicDevices] = useState<InputDevice[]>([])
  const [mixSystemDevices, setMixSystemDevices] = useState<InputDevice[]>([])
  const [audioProcesses, setAudioProcesses] = useState<AudioProcessInfo[]>([])
  const [processesLoading, setProcessesLoading] = useState(false)
  const [processError, setProcessError] = useState<string | null>(null)
  const [devicesLoading, setDevicesLoading] = useState(false)
  const [deviceError, setDeviceError] = useState<string | null>(null)

  const [overlayCfg, setOverlayCfg] = useState<OverlayConfig | null>(null)
  const [overlayVisible, setOverlayVisible] = useState(false)

  const [hotkeys, setHotkeys] = useState<HotkeyInfo[]>([])
  const [hotkeyProblems, setHotkeyProblems] = useState<HotkeyFailure[]>([])

  const [onboardingDone, setOnboardingDone] = useState(false)
  const [analyticsEnabled, setAnalyticsEnabledState] = useState(true)
  const [permissions, setPermissions] = useState<PermissionSnapshot | null>(null)
  const [speechStatus, setSpeechStatus] = useState<MacSpeechStatus | null>(null)
  const [systemAudioProbe, setSystemAudioProbe] = useState<SystemAudioProbe | null>(null)
  const [permissionError, setPermissionError] = useState<CmdPermissionError | null>(null)

  const [errorState, setErrorState] = useState<{
    message: string
    settingsPane: SystemSettingsPane | null
  } | null>(null)
  const error = errorState?.message ?? null
  const errorSettingsPane = errorState?.settingsPane ?? null
  const setError = useCallback((message: string | null) => {
    setErrorState(message === null ? null : { message, settingsPane: null })
  }, [])
  const [pending, setPending] = useState(false)
  const pendingRef = useRef(false)

  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResult | null>(null)
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>('idle')
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const [installedVersion, setInstalledVersion] = useState<string | null>(null)
  const updateBusyRef = useRef(false)

  useEffect(() => {
    appInfo()
      .then((i) => {
        setInfo(i)
        setTrackingTier(i.analyticsTier)
      })
      .catch((e) => setError(formatCmdError(e)))
    void initLocale()
    hotkeyFailures()
      .then(setHotkeyProblems)
      .catch(() => {})
    shortcutsGet()
      .then(setHotkeys)
      .catch(() => {})

    const unlisten: Array<() => void> = []
    let disposed = false
    const keep = (fn: () => void) => {
      if (disposed) fn()
      else unlisten.push(fn)
    }

    on<TaskStatus>(EVENT.taskState, (s) => {
      setStatus(s)
      if (s !== TaskStatus.Capturing) {
        setAudio(null)
        setLanguageMismatch(false)
      }
    }).then(keep)

    on<AudioStatus>(EVENT.audio, setAudio).then(keep)

    on<boolean>(EVENT.languageMismatch, setLanguageMismatch).then(keep)

    on<OverlayConfig>(EVENT.overlayState, setOverlayCfg).then(keep)

    on<boolean>(EVENT.overlayVisibility, setOverlayVisible).then(keep)

    on<UpdateProgress>(EVENT.updateProgress, setUpdateProgress).then(keep)

    return () => {
      disposed = true
      for (const fn of unlisten) fn()
    }
  }, [setError])

  useEffect(() => {
    prefsGet()
      .then((p) => {
        setAudioSource(p.audioSource as AudioSourceKind)
        setTranslateEngine(p.translateEngine)
        setAsrEngine(p.asrEngine)
        setSourceLang(p.sourceLang)
        setTargetLang(p.targetLang)
        setZhScript(p.zhScript)
        setTheme(p.theme)
        setLanguage(p.language)
        setEndpoint(p.endpoint)
        setOnboardingDone(p.onboardingDone)
        setAnalyticsEnabledState(p.analyticsEnabled)
        setTrackingEnabled(p.analyticsEnabled)
        if (p.deviceId) setDevice(p.deviceId)
        if (p.mixMicDeviceId) setMixMicDevice(p.mixMicDeviceId)
        if (p.mixSystemDeviceId) setMixSystemDevice(p.mixSystemDeviceId)
        setAppTargetsState(p.appTargets ?? [])
      })
      .catch(() => {})
      .finally(() => setPrefsReady(true))

    overlayConfigGet()
      .then(setOverlayCfg)
      .catch(() => {})
    overlayIsVisible()
      .then(setOverlayVisible)
      .catch(() => {})
  }, [])

  const refreshPermission = useCallback(async () => {
    try {
      setPermissions(await permissionStatus())
    } catch {
      setPermissions(null)
    }
  }, [])

  const refreshSpeechStatus = useCallback(async () => {
    if (!info?.macSpeechSupported) {
      setSpeechStatus(null)
      return
    }
    try {
      setSpeechStatus(await macSpeechStatus())
    } catch {
      setSpeechStatus(null)
    }
  }, [info?.macSpeechSupported])

  const runSystemAudioProbe = useCallback(async () => {
    const result = await permissionProbeSystemAudio()
    setSystemAudioProbe(result)
    return result
  }, [])

  useEffect(() => {
    void refreshPermission()
    void refreshSpeechStatus()
    const onFocus = () => {
      void refreshPermission()
      void refreshSpeechStatus()
      setSystemAudioProbe(null)
    }
    window.addEventListener('focus', onFocus)
    return () => window.removeEventListener('focus', onFocus)
  }, [refreshPermission, refreshSpeechStatus])

  const applyTheme = useCallback((pref: Theme) => {
    const systemDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    const dark = pref === Theme.Dark || (pref === Theme.System && systemDark)
    document.documentElement.classList.toggle('theme-light', !dark)
    try {
      localStorage.setItem('audioscript.theme', pref)
    } catch {}
  }, [])

  useEffect(() => {
    applyTheme(theme)
    if (theme !== Theme.System) return
    const mq = window.matchMedia('(prefers-color-scheme: dark)')
    const onChange = () => applyTheme(Theme.System)
    mq.addEventListener('change', onChange)
    return () => mq.removeEventListener('change', onChange)
  }, [theme, applyTheme])

  const patchPrefs = useCallback(
    async (patch: PrefsPatch) => {
      try {
        const p = await prefsSet(patch)
        if (patch.theme !== undefined) setTheme(p.theme)
        if (patch.asrEngine !== undefined) setAsrEngine(p.asrEngine)
        if (patch.endpoint !== undefined) setEndpoint(p.endpoint)
      } catch (e) {
        setError(formatCmdError(e))
      }
    },
    [setError],
  )

  const completeOnboarding = useCallback(
    async (from: 'start' | 'later') => {
      setOnboardingDone(true)
      try {
        await prefsSet({ onboardingDone: true })
        track('onboarding_completed', { from })
      } catch (e) {
        setError(formatCmdError(e))
      }
    },
    [setError],
  )

  const setAnalyticsEnabled = useCallback(
    (next: boolean) => {
      if (!next) track('setting_changed', { key: 'analytics', value: 'off' })
      setAnalyticsEnabledState(next)
      setTrackingEnabled(next)
      void patchPrefs({ analyticsEnabled: next })
      if (next) track('setting_changed', { key: 'analytics', value: 'on' })
    },
    [patchPrefs],
  )

  const devicesLoadingRef = useRef(false)
  const refreshDevices = useCallback(async () => {
    if (audioSource === AudioSourceKind.App) {
      setDevices([])
      setDeviceError(null)
      setDevicesLoading(false)
      return
    }
    if (devicesLoadingRef.current) return
    devicesLoadingRef.current = true
    setDevicesLoading(true)
    const load = async (
      source: AudioSourceKind,
      label: string,
      setList: (list: InputDevice[]) => void,
      setSelected: (update: (cur: string) => string) => void,
    ): Promise<string | null> => {
      try {
        const list = await audioDevices(source)
        setList(list)
        setSelected((cur) => cur || (list.find((d) => d.isDefault)?.id ?? ''))
        return null
      } catch (e) {
        return t('common.labeledError', { label, error: formatCmdError(e) })
      }
    }

    try {
      if (audioSource === AudioSourceKind.Mix) {
        const errors = (
          await Promise.all([
            load(AudioSourceKind.Mic, t('permissions.mic'), setMixMicDevices, setMixMicDevice),
            load(AudioSourceKind.Loopback, t('audio.sourceLoopbackLabel'), setMixSystemDevices, setMixSystemDevice),
          ])
        ).filter((error): error is string => error !== null)
        setDeviceError(errors.length > 0 ? errors.join('\n') : null)
      } else {
        const list = await audioDevices(audioSource)
        setDevices(list)
        setDeviceError(null)
        setDevice((cur) => cur || (list.find((d) => d.isDefault)?.id ?? ''))
      }
    } catch (e) {
      setDeviceError(formatCmdError(e))
    } finally {
      devicesLoadingRef.current = false
      setDevicesLoading(false)
    }
  }, [audioSource])

  const processesLoadingRef = useRef(false)
  const refreshProcesses = useCallback(async () => {
    if (processesLoadingRef.current) return
    processesLoadingRef.current = true
    setProcessesLoading(true)
    const started = Date.now()
    let ok = false
    try {
      setAudioProcesses(await fetchAudioProcesses())
      setProcessError(null)
      ok = true
    } catch (e) {
      setAudioProcesses([])
      setProcessError(formatCmdError(e))
    } finally {
      if (ok) {
        const remaining = PROCESS_REFRESH_MIN_MS - (Date.now() - started)
        if (remaining > 0) await new Promise((resolve) => setTimeout(resolve, remaining))
      }
      processesLoadingRef.current = false
      setProcessesLoading(false)
    }
  }, [])

  useEffect(() => {
    if (!prefsReady) return
    refreshDevices()
    if (audioSource === AudioSourceKind.App) void refreshProcesses()
  }, [prefsReady, audioSource, refreshDevices, refreshProcesses])

  const toggleSession = useCallback(async () => {
    if (pendingRef.current) return
    pendingRef.current = true
    setPending(true)
    setError(null)
    setPermissionError(null)
    const starting = status !== TaskStatus.Capturing
    try {
      if (status === TaskStatus.Capturing) {
        setStatus(await withTimeout(sessionStop(), STOP_TIMEOUT_MS, t('error.stopTimeout')))
        setActiveDevice(null)
      } else {
        if (audioSource === AudioSourceKind.App && appTargets.length === 0) {
          setError(t('audio.appRequired'))
          return
        }
        const selection =
          audioSource === AudioSourceKind.Mix
            ? { mixMicDeviceId: mixMicDevice, mixSystemDeviceId: mixSystemDevice }
            : {
                device,
                appTargets: audioSource === AudioSourceKind.App ? appTargets : [],
              }
        setStatus(
          await withTimeout(
            sessionStart(selection, translateEngine, sourceLang, targetLang, zhScript, audioSource),
            START_TIMEOUT_MS,
            t('error.startTimeout'),
          ),
        )
        setActiveDevice((await sessionStatus()).device ?? null)
        void refreshPermission()
      }
    } catch (e) {
      const denied = permissionOf(e)
      const settings = settingsOf(e)
      if (starting) {
        track('session_start_failed', {
          origin: 'ui',
          audio_source: audioSource,
          translate_engine: translateEngine,
          asr_engine: asrEngine,
          error_kind: denied ? 'permission' : settings ? 'settings' : typeof e === 'string' ? 'timeout' : 'failed',
          permission_kind: denied?.permission ?? 'none',
          settings_pane: settings?.pane ?? 'none',
        })
      }
      if (denied) {
        setPermissionError(denied)
        void refreshPermission()
      } else {
        setErrorState({ message: formatCmdError(e), settingsPane: settings?.pane ?? null })
      }
      sessionStatus()
        .then((i) => setStatus(i.status))
        .catch(() => {})
    } finally {
      pendingRef.current = false
      setPending(false)
    }
  }, [
    status,
    device,
    mixMicDevice,
    mixSystemDevice,
    appTargets,
    translateEngine,
    asrEngine,
    sourceLang,
    targetLang,
    zhScript,
    audioSource,
    refreshPermission,
    setError,
  ])

  const clearAll = useCallback(async () => {
    await sessionClear()
  }, [])

  const updateOverlay = useCallback(
    async (patch: OverlayConfigPatch) => {
      if (!overlayCfg) return
      try {
        setOverlayCfg(await overlayConfigPatch(patch))
      } catch (e) {
        setError(formatCmdError(e))
      }
    },
    [overlayCfg, setError],
  )

  const toggleOverlay = useCallback(async () => {
    try {
      setOverlayVisible(await overlayToggle())
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [setError])

  const refreshHotkeys = useCallback(async () => {
    try {
      setHotkeys(await shortcutsGet())
      setHotkeyProblems(await hotkeyFailures())
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [setError])

  const changeTheme = useCallback(
    async (next: Theme) => {
      setTheme(next)
      await patchPrefs({ theme: next })
      track('setting_changed', { key: 'theme', value: next })
    },
    [patchPrefs],
  )

  const changeAudioSource = useCallback(
    (v: AudioSourceKind) => {
      setAudioSource(v)
      setDevice('')
      void patchPrefs({ audioSource: v, deviceId: '' })
      track('setting_changed', { key: 'audio_source', value: v })
    },
    [patchPrefs],
  )

  const changeDevice = useCallback(
    (v: string) => {
      setDevice(v)
      void patchPrefs({ deviceId: v })
    },
    [patchPrefs],
  )

  const changeMixMicDevice = useCallback(
    (v: string) => {
      setMixMicDevice(v)
      void patchPrefs({ mixMicDeviceId: v })
    },
    [patchPrefs],
  )

  const changeMixSystemDevice = useCallback(
    (v: string) => {
      setMixSystemDevice(v)
      void patchPrefs({ mixSystemDeviceId: v })
    },
    [patchPrefs],
  )

  const changeAppTargets = useCallback(
    (v: AppRef[]) => {
      setAppTargetsState(v)
      void patchPrefs({ appTargets: v })
      const value = v.length === 0 ? 'none' : v.length === 1 ? 'single' : 'multi'
      track('setting_changed', { key: 'app_target', value })
    },
    [patchPrefs],
  )

  const changeTranslateEngine = useCallback(
    (v: TranslateEngine) => {
      setTranslateEngine(v)
      void patchPrefs({ translateEngine: v })
      track('setting_changed', { key: 'translate_engine', value: v })
    },
    [patchPrefs],
  )

  const changeAsrEngine = useCallback(
    (v: EngineType) => {
      setAsrEngine(v)
      void patchPrefs({ asrEngine: v })
      track('setting_changed', { key: 'asr_engine', value: v })
    },
    [patchPrefs],
  )

  const changeEngineAndDirection = useCallback(
    (engine: TranslateEngine, source: string, target: string, script: ZhScript) => {
      setTranslateEngine(engine)
      setSourceLang(source)
      setTargetLang(target)
      setZhScript(script)
      void patchPrefs({ translateEngine: engine, sourceLang: source, targetLang: target, zhScript: script })
      track('setting_changed', { key: 'translate_engine', value: engine })
      track('setting_changed', { key: 'translate_direction', value: `${source}:${target}:${script}` })
    },
    [patchPrefs],
  )

  const changeDirection = useCallback(
    (source: string, target: string, script: ZhScript) => {
      setSourceLang(source)
      setTargetLang(target)
      setZhScript(script)
      void patchPrefs({ sourceLang: source, targetLang: target, zhScript: script })
      track('setting_changed', { key: 'translate_direction', value: `${source}:${target}:${script}` })
    },
    [patchPrefs],
  )

  const changeEndpoint = useCallback(
    async (v: EndpointConfig) => {
      setEndpoint(v)
      await patchPrefs({ endpoint: v })
    },
    [patchPrefs],
  )

  const changeLanguage = useCallback(
    async (v: Language) => {
      setLanguage(v)
      await patchPrefs({ language: v })
      track('setting_changed', { key: 'language', value: v })
    },
    [patchPrefs],
  )

  const checkUpdate = useCallback(async (silent = false) => {
    if (updateBusyRef.current) return
    updateBusyRef.current = true
    setUpdatePhase('checking')
    if (!silent) setUpdateError(null)
    try {
      const result = await updateCheck()
      setUpdateInfo(result)
      track('update_checked', {
        available: result.available ? 1 : 0,
        manual: silent ? 0 : 1,
        failed: 0,
      })
      setUpdatePhase('idle')
    } catch (e) {
      track('update_checked', { available: 0, manual: silent ? 0 : 1, failed: 1 })
      setUpdatePhase(silent ? 'idle' : 'error')
      if (!silent) setUpdateError(formatCmdError(e))
    } finally {
      updateBusyRef.current = false
    }
  }, [])

  const installUpdate = useCallback(async () => {
    if (updateBusyRef.current) return
    updateBusyRef.current = true
    setUpdatePhase('downloading')
    setUpdateError(null)
    setUpdateProgress({ downloaded: 0, total: null })
    track('update_prompt', { action: 'install' })
    try {
      const r = await updateInstall()
      setInstalledVersion(r.version)
      setUpdatePhase('ready')
      track('update_install', { result: 'ok' })
    } catch (e) {
      setUpdatePhase('error')
      setUpdateError(formatCmdError(e))
      track('update_install', { result: 'failed' })
    } finally {
      updateBusyRef.current = false
    }
  }, [])

  const restartApp = useCallback(async () => {
    try {
      await updateRestart()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [setError])

  const autoCheckedRef = useRef(false)
  useEffect(() => {
    if (!info || autoCheckedRef.current) return
    autoCheckedRef.current = true
    if (!import.meta.env.PROD) return
    void checkUpdate(true)
  }, [info, checkUpdate])

  const value = useMemo<Store>(
    () => ({
      info,
      updateInfo,
      updatePhase,
      updateProgress,
      updateError,
      installedVersion,
      checkUpdate,
      installUpdate,
      restartApp,
      prefsReady,
      devicesLoading,
      audioSource,
      device,
      mixMicDevice,
      mixSystemDevice,
      appTargets,
      translateEngine,
      asrEngine,
      sourceLang,
      targetLang,
      zhScript,
      theme,
      language,
      endpoint,
      status,
      activeDevice,
      audio,
      languageMismatch,
      setLanguageMismatch,
      devices,
      mixMicDevices,
      mixSystemDevices,
      audioProcesses,
      processesLoading,
      processError,
      deviceError,
      overlayCfg,
      overlayVisible,
      hotkeys,
      hotkeyProblems,
      onboardingDone,
      completeOnboarding,
      analyticsEnabled,
      setAnalyticsEnabled,
      permissions,
      refreshPermission,
      speechStatus,
      setSpeechStatus,
      refreshSpeechStatus,
      systemAudioProbe,
      runSystemAudioProbe,
      permissionError,
      setPermissionError,
      error,
      errorSettingsPane,
      setError,
      pending,
      toggleSession,
      clearAll,
      setAudioSource: changeAudioSource,
      setDevice: changeDevice,
      setMixMicDevice: changeMixMicDevice,
      setMixSystemDevice: changeMixSystemDevice,
      setAppTargets: changeAppTargets,
      setTranslateEngine: changeTranslateEngine,
      setAsrEngine: changeAsrEngine,
      setEngineAndDirection: changeEngineAndDirection,
      setDirection: changeDirection,
      changeTheme,
      changeLanguage,
      setEndpoint: changeEndpoint,
      updateOverlay,
      toggleOverlay,
      refreshDevices,
      refreshProcesses,
      refreshHotkeys,
    }),
    [
      info,
      updateInfo,
      updatePhase,
      updateProgress,
      updateError,
      installedVersion,
      checkUpdate,
      installUpdate,
      restartApp,
      prefsReady,
      devicesLoading,
      audioSource,
      device,
      mixMicDevice,
      mixSystemDevice,
      appTargets,
      translateEngine,
      asrEngine,
      sourceLang,
      targetLang,
      zhScript,
      theme,
      language,
      endpoint,
      status,
      activeDevice,
      audio,
      languageMismatch,
      devices,
      mixMicDevices,
      mixSystemDevices,
      audioProcesses,
      processesLoading,
      processError,
      deviceError,
      overlayCfg,
      overlayVisible,
      hotkeys,
      hotkeyProblems,
      onboardingDone,
      completeOnboarding,
      analyticsEnabled,
      setAnalyticsEnabled,
      permissions,
      refreshPermission,
      speechStatus,
      refreshSpeechStatus,
      systemAudioProbe,
      runSystemAudioProbe,
      permissionError,
      error,
      errorSettingsPane,
      setError,
      pending,
      toggleSession,
      clearAll,
      changeAudioSource,
      changeDevice,
      changeMixMicDevice,
      changeMixSystemDevice,
      changeAppTargets,
      changeTranslateEngine,
      changeAsrEngine,
      changeEngineAndDirection,
      changeDirection,
      changeTheme,
      changeLanguage,
      changeEndpoint,
      updateOverlay,
      toggleOverlay,
      refreshDevices,
      refreshProcesses,
      refreshHotkeys,
    ],
  )

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}
