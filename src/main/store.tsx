import { createContext, type ReactNode, useCallback, useContext, useEffect, useMemo, useRef, useState } from 'react'
import { setTrackingEnabled, track } from '@/shared/analytics'
import {
  type AppInfo,
  AudioSourceKind,
  type AudioStatus,
  appInfo,
  audioDevices,
  type CmdPermissionError,
  EVENT,
  formatCmdError,
  type HotkeyFailure,
  type HotkeyInfo,
  hotkeyFailures,
  type InputDevice,
  type OverlayConfig,
  on,
  overlayConfigGet,
  overlayConfigSet,
  overlayIsVisible,
  overlayToggle,
  type PermissionSnapshot,
  type Preferences,
  type PrefsPatch,
  permissionOf,
  permissionStatus,
  prefsGet,
  prefsSet,
  sessionClear,
  sessionStart,
  sessionStatus,
  sessionStop,
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
import { withTimeout } from '@/shared/timeout'
import { TaskStatus } from '@/shared/types'

type EndpointConfig = Preferences['endpoint']

export type UpdatePhase = 'idle' | 'checking' | 'downloading' | 'ready' | 'error'

interface Store {
  info: AppInfo | null

  prefsReady: boolean
  audioSource: AudioSourceKind
  device: string
  translateEngine: TranslateEngine
  sourceLang: string
  zhScript: ZhScript
  theme: Theme
  endpoint: EndpointConfig

  status: TaskStatus
  activeDevice: string | null
  audio: AudioStatus | null

  devices: InputDevice[]
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
  completeOnboarding: () => Promise<void>

  analyticsEnabled: boolean
  setAnalyticsEnabled: (v: boolean) => void

  permissions: PermissionSnapshot | null
  refreshPermission: () => Promise<void>
  permissionError: CmdPermissionError | null
  setPermissionError: (e: CmdPermissionError | null) => void

  error: string | null
  setError: (e: string | null) => void
  pending: boolean

  toggleSession: () => Promise<void>
  clearAll: () => Promise<void>
  setAudioSource: (v: AudioSourceKind) => void
  setDevice: (v: string) => void
  setTranslateEngine: (v: TranslateEngine) => void
  setDirection: (lang: string, script: ZhScript) => void
  changeTheme: (v: Theme) => Promise<void>
  setEndpoint: (v: EndpointConfig) => Promise<void>
  updateOverlay: (patch: Partial<OverlayConfig>) => Promise<void>
  toggleOverlay: () => Promise<void>
  refreshDevices: () => Promise<void>
  refreshHotkeys: () => Promise<void>
}

const STOP_TIMEOUT_MS = 8_000

const START_TIMEOUT_MS = 60_000

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
  const [translateEngine, setTranslateEngine] = useState<TranslateEngine>(TranslateEngine.LocalNmt)
  const [sourceLang, setSourceLang] = useState('en')
  const [zhScript, setZhScript] = useState(ZhScript.Simplified)
  const [theme, setTheme] = useState<Theme>(Theme.System)
  const [endpoint, setEndpoint] = useState<EndpointConfig>({
    trailingSilenceMs: 800,
    maxUtteranceMs: 8000,
  })

  const [status, setStatus] = useState<TaskStatus>(TaskStatus.Ready)
  const [activeDevice, setActiveDevice] = useState<string | null>(null)
  const [audio, setAudio] = useState<AudioStatus | null>(null)
  const sessionVersion = useRef(0)

  const [devices, setDevices] = useState<InputDevice[]>([])
  const [deviceError, setDeviceError] = useState<string | null>(null)

  const [overlayCfg, setOverlayCfg] = useState<OverlayConfig | null>(null)
  const [overlayVisible, setOverlayVisible] = useState(false)

  const [hotkeys, setHotkeys] = useState<HotkeyInfo[]>([])
  const [hotkeyProblems, setHotkeyProblems] = useState<HotkeyFailure[]>([])

  const [onboardingDone, setOnboardingDone] = useState(false)
  const [analyticsEnabled, setAnalyticsEnabledState] = useState(false)
  const [permissions, setPermissions] = useState<PermissionSnapshot | null>(null)
  const [permissionError, setPermissionError] = useState<CmdPermissionError | null>(null)

  const [error, setError] = useState<string | null>(null)
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
      .then(setInfo)
      .catch((e) => setError(formatCmdError(e)))
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

    const reportError = (e: unknown) => {
      if (!disposed) setError(formatCmdError(e))
    }
    const refreshSession = async () => {
      const version = ++sessionVersion.current
      try {
        const current = await sessionStatus()
        if (disposed || version !== sessionVersion.current) return
        setStatus(current.status)
        setActiveDevice(current.device ?? null)
        if (current.status !== TaskStatus.Capturing) setAudio(null)
        if (current.error) setError(current.error)
      } catch (e) {
        if (version === sessionVersion.current) reportError(e)
      }
    }

    on<TaskStatus>(EVENT.taskState, (s) => {
      if (disposed) return
      setStatus(s)
      if (s !== TaskStatus.Capturing) {
        setAudio(null)
        setActiveDevice(null)
      }
      void refreshSession()
    })
      .then((fn) => {
        keep(fn)
        if (!disposed) void refreshSession()
      })
      .catch(reportError)

    on<string>(EVENT.sessionError, (message) => {
      if (disposed) return
      sessionVersion.current++
      setStatus(TaskStatus.Error)
      setActiveDevice(null)
      setAudio(null)
      setError(message)
    })
      .then(keep)
      .catch(reportError)

    on<AudioStatus>(EVENT.audio, setAudio).then(keep).catch(reportError)

    on<OverlayConfig>(EVENT.overlayState, setOverlayCfg).then(keep).catch(reportError)

    on<boolean>(EVENT.overlayVisibility, setOverlayVisible).then(keep).catch(reportError)

    on<UpdateProgress>(EVENT.updateProgress, setUpdateProgress).then(keep).catch(reportError)

    return () => {
      disposed = true
      sessionVersion.current++
      for (const fn of unlisten) fn()
    }
  }, [])

  useEffect(() => {
    prefsGet()
      .then((p) => {
        setAudioSource(p.audioSource as AudioSourceKind)
        setTranslateEngine(p.translateEngine)
        setSourceLang(p.sourceLang)
        setZhScript(p.zhScript)
        setTheme(p.theme)
        setEndpoint(p.endpoint)
        setOnboardingDone(p.onboardingDone)
        setAnalyticsEnabledState(p.analyticsEnabled)
        if (p.deviceId) setDevice(p.deviceId)
      })
      .catch((e) => setError(`读取设置失败：${formatCmdError(e)}`))
      .finally(() => setPrefsReady(true))

    overlayConfigGet()
      .then(setOverlayCfg)
      .catch(() => {})
    overlayIsVisible()
      .then(setOverlayVisible)
      .catch(() => {})
  }, [])

  useEffect(() => {
    setTrackingEnabled(prefsReady && analyticsEnabled && info?.analyticsAvailable === true)
  }, [prefsReady, analyticsEnabled, info])

  const refreshPermission = useCallback(async () => {
    try {
      setPermissions(await permissionStatus())
    } catch {
      setPermissions(null)
    }
  }, [])

  useEffect(() => {
    void refreshPermission()
    const onFocus = () => void refreshPermission()
    window.addEventListener('focus', onFocus)
    return () => window.removeEventListener('focus', onFocus)
  }, [refreshPermission])

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

  const patchPrefs = useCallback(async (patch: PrefsPatch) => {
    try {
      const p = await prefsSet(patch)
      if (patch.theme !== undefined) setTheme(p.theme)
      if (patch.endpoint !== undefined) setEndpoint(p.endpoint)
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [])

  const completeOnboarding = useCallback(async () => {
    setOnboardingDone(true)
    try {
      await prefsSet({ onboardingDone: true })
      track('onboarding_completed')
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [])

  const setAnalyticsEnabled = useCallback(
    (next: boolean) => {
      if (next && !info?.analyticsAvailable) {
        setError(info?.analyticsDisabledReason ?? '此构建未配置匿名统计')
        return
      }
      void prefsSet({ analyticsEnabled: next })
        .then((p) => {
          setAnalyticsEnabledState(p.analyticsEnabled)
          setTrackingEnabled(p.analyticsEnabled && info?.analyticsAvailable === true)
          if (p.analyticsEnabled) track('setting_changed', { key: 'analytics', value: 'on' })
        })
        .catch((e) => setError(formatCmdError(e)))
    },
    [info],
  )

  const refreshDevices = useCallback(async () => {
    try {
      const list = await audioDevices(audioSource)
      setDevices(list)
      setDeviceError(null)
      setDevice((cur) => cur || (list.find((d) => d.isDefault)?.id ?? ''))
    } catch (e) {
      setDeviceError(formatCmdError(e))
    }
  }, [audioSource])

  useEffect(() => {
    if (!prefsReady) return
    refreshDevices()
  }, [prefsReady, refreshDevices])

  const toggleSession = useCallback(async () => {
    if (pendingRef.current) return
    pendingRef.current = true
    setPending(true)
    setError(null)
    setPermissionError(null)
    try {
      if (status === TaskStatus.Capturing) {
        await withTimeout(sessionStop(), STOP_TIMEOUT_MS, '停止请求超时：请重试，或使用托盘「停止转写」')
      } else {
        await withTimeout(
          sessionStart(device || undefined, translateEngine, sourceLang, zhScript, audioSource),
          START_TIMEOUT_MS,
          '开始转写超时，请重试',
        )
        void refreshPermission()
      }
      const version = ++sessionVersion.current
      const current = await sessionStatus()
      if (version === sessionVersion.current) {
        setStatus(current.status)
        setActiveDevice(current.device ?? null)
        if (current.status !== TaskStatus.Capturing) setAudio(null)
        if (current.error) setError(current.error)
      }
    } catch (e) {
      const denied = permissionOf(e)
      if (denied) {
        setPermissionError(denied)
        void refreshPermission()
      } else {
        setError(formatCmdError(e))
      }
      const version = ++sessionVersion.current
      sessionStatus()
        .then((i) => {
          if (version !== sessionVersion.current) return
          setStatus(i.status)
          setActiveDevice(i.device ?? null)
          if (i.error) setError(i.error)
        })
        .catch(() => {})
    } finally {
      pendingRef.current = false
      setPending(false)
    }
  }, [status, device, translateEngine, sourceLang, zhScript, audioSource, refreshPermission])

  const clearAll = useCallback(async () => {
    await sessionClear()
  }, [])

  const updateOverlay = useCallback(
    async (patch: Partial<OverlayConfig>) => {
      if (!overlayCfg) return
      try {
        setOverlayCfg(await overlayConfigSet({ ...overlayCfg, ...patch }))
      } catch (e) {
        setError(formatCmdError(e))
      }
    },
    [overlayCfg],
  )

  const toggleOverlay = useCallback(async () => {
    try {
      setOverlayVisible(await overlayToggle())
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [])

  const refreshHotkeys = useCallback(async () => {
    try {
      setHotkeys(await shortcutsGet())
      setHotkeyProblems(await hotkeyFailures())
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [])

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

  const changeTranslateEngine = useCallback(
    (v: TranslateEngine) => {
      setTranslateEngine(v)
      void patchPrefs({ translateEngine: v })
      track('setting_changed', { key: 'translate_engine', value: v })
    },
    [patchPrefs],
  )

  const changeDirection = useCallback(
    (lang: string, script: ZhScript) => {
      setSourceLang(lang)
      setZhScript(script)
      void patchPrefs({ sourceLang: lang, zhScript: script })
      track('setting_changed', { key: 'translate_direction', value: `${lang}:${script}` })
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

  const checkUpdate = useCallback(async (silent = false) => {
    if (updateBusyRef.current) return
    updateBusyRef.current = true
    setUpdatePhase('checking')
    if (!silent) setUpdateError(null)
    try {
      setUpdateInfo(await updateCheck())
      setUpdatePhase('idle')
    } catch (e) {
      setUpdatePhase(silent ? 'idle' : 'error')
      if (!silent) setUpdateError(formatCmdError(e))
    } finally {
      updateBusyRef.current = false
    }
  }, [])

  const installUpdate = useCallback(async () => {
    if (!info?.updaterEnabled || !updateInfo?.enabled || !updateInfo.available) {
      setUpdateError(info?.updaterDisabledReason ?? updateInfo?.disabledReason ?? '当前没有可安装的更新')
      setUpdatePhase('error')
      return
    }
    if (updateBusyRef.current) return
    updateBusyRef.current = true
    setUpdatePhase('downloading')
    setUpdateError(null)
    setUpdateProgress({ downloaded: 0, total: null })
    try {
      const r = await updateInstall()
      setInstalledVersion(r.version)
      setUpdatePhase('ready')
    } catch (e) {
      setUpdatePhase('error')
      setUpdateError(formatCmdError(e))
    } finally {
      updateBusyRef.current = false
    }
  }, [info, updateInfo])

  const restartApp = useCallback(async () => {
    try {
      await updateRestart()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }, [])

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
      audioSource,
      device,
      translateEngine,
      sourceLang,
      zhScript,
      theme,
      endpoint,
      status,
      activeDevice,
      audio,
      devices,
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
      permissionError,
      setPermissionError,
      error,
      setError,
      pending,
      toggleSession,
      clearAll,
      setAudioSource: changeAudioSource,
      setDevice: changeDevice,
      setTranslateEngine: changeTranslateEngine,
      setDirection: changeDirection,
      changeTheme,
      setEndpoint: changeEndpoint,
      updateOverlay,
      toggleOverlay,
      refreshDevices,
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
      audioSource,
      device,
      translateEngine,
      sourceLang,
      zhScript,
      theme,
      endpoint,
      status,
      activeDevice,
      audio,
      devices,
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
      permissionError,
      error,
      pending,
      toggleSession,
      clearAll,
      changeAudioSource,
      changeDevice,
      changeTranslateEngine,
      changeDirection,
      changeTheme,
      changeEndpoint,
      updateOverlay,
      toggleOverlay,
      refreshDevices,
      refreshHotkeys,
    ],
  )

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}
