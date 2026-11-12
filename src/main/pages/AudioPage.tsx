import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { useStore } from '@/main/store'
import { useI18n } from '@/shared/i18n'
import {
  AudioSourceKind,
  formatCmdError,
  type InputDevice,
  type MonitorTarget,
  monitorStart,
  monitorStop,
  openSystemSettings,
  permissionOf,
} from '@/shared/ipc'
import { isMacOS, isWindows } from '@/shared/platform'
import { TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { AppPicker, type AppPickerOption } from '@/ui/AppPicker'
import { Button } from '@/ui/Button'
import { ChoiceCard, ChoiceGroup } from '@/ui/ChoiceCard'
import { DeviceSelect, describeDevice } from '@/ui/DeviceSelect'
import { DeviceLevelMeter, LevelMeter } from '@/ui/LevelMeter'
import { Page } from '@/ui/Page'
import { SectionLabel, SettingsGroup, SettingsRow } from '@/ui/Settings'

const MAX_APP_TARGETS = 4

export function AudioPage({ onOpenPermissions }: { onOpenPermissions: () => void }) {
  const {
    audioSource,
    setAudioSource,
    devices,
    device,
    setDevice,
    mixMicDevices,
    mixSystemDevices,
    mixMicDevice,
    mixSystemDevice,
    setMixMicDevice,
    setMixSystemDevice,
    appTargets,
    setAppTargets,
    audioProcesses,
    processesLoading,
    processError,
    deviceError,
    refreshDevices,
    refreshProcesses,
    audio,
    status,
    pending,
    prefsReady,
    devicesLoading,
    info,
  } = useStore()
  const { t } = useI18n()

  const appSupported = info?.appAudioSupported !== false
  const sourceOptions = useMemo(
    () =>
      [
        {
          value: AudioSourceKind.Loopback,
          label: t('audio.sourceLoopbackLabel'),
          description: t('audio.sourceLoopbackDescription'),
        },
        {
          value: AudioSourceKind.App,
          label: t('audio.sourceAppLabel'),
          description: t('audio.sourceAppDescription'),
        },
        {
          value: AudioSourceKind.Mic,
          label: t('audio.sourceMicLabel'),
          description: t('audio.sourceMicDescription'),
        },
        {
          value: AudioSourceKind.Mix,
          label: t('audio.sourceMixLabel'),
          description: t('audio.sourceMixDescription'),
        },
      ].filter((option) => option.value !== AudioSourceKind.App || appSupported),
    [t, appSupported],
  )

  const capturing = status === TaskStatus.Capturing
  const isMix = audioSource === AudioSourceKind.Mix
  const isApp = audioSource === AudioSourceKind.App && appSupported

  const [showAllApps, setShowAllApps] = useState(false)
  const [previewOn, setPreviewOn] = useState(false)
  const [openMenu, setOpenMenu] = useState<'single' | 'mic' | 'system' | null>(null)
  const [monitorError, setMonitorError] = useState<{ message: string; permission: boolean } | null>(null)
  const [dismissedDeviceError, setDismissedDeviceError] = useState<string | null>(null)
  const previewSeq = useRef(0)

  useEffect(() => {
    if (!deviceError) setDismissedDeviceError(null)
  }, [deviceError])
  const previewing = previewOn && !pending && !capturing

  useEffect(() => {
    if (!isApp || !prefsReady) return
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') void refreshProcesses(true)
    }, 2000)
    return () => window.clearInterval(timer)
  }, [isApp, prefsReady, refreshProcesses])

  const targets = useMemo<MonitorTarget[]>(() => {
    const out: MonitorTarget[] = []
    if (isMix) {
      out.push({ source: AudioSourceKind.Mic, device: mixMicDevice })
      out.push({ source: AudioSourceKind.Loopback, device: mixSystemDevice })
      if (openMenu === 'mic') {
        for (const d of mixMicDevices) out.push({ source: AudioSourceKind.Mic, device: d.id })
      }
      if (openMenu === 'system') {
        for (const d of mixSystemDevices) out.push({ source: AudioSourceKind.Loopback, device: d.id })
      }
      return out
    }
    if (isApp) {
      return appTargets.length > 0 ? [{ source: AudioSourceKind.App, device: null, apps: appTargets }] : []
    }
    if (openMenu === 'single') {
      for (const d of devices) out.push({ source: audioSource, device: d.id })
      if (!device) out.push({ source: audioSource, device: null })
      if (out.length === 0) out.push({ source: audioSource, device: null })
      return out
    }
    out.push({ source: audioSource, device })
    return out
  }, [
    isMix,
    isApp,
    appTargets,
    mixMicDevice,
    mixSystemDevice,
    openMenu,
    mixMicDevices,
    mixSystemDevices,
    devices,
    device,
    audioSource,
  ])

  const visibleProcesses = useMemo(
    () => (showAllApps ? audioProcesses : audioProcesses.filter((p) => p.isOutputActive === true)),
    [audioProcesses, showAllApps],
  )

  const selectedProcesses = useMemo(
    () =>
      appTargets.map((app) => ({
        app,
        process: visibleProcesses.find((p) => p.key === app.key || p.name === app.name),
      })),
    [appTargets, visibleProcesses],
  )
  const selectedKeys = selectedProcesses.map(({ app, process }) => process?.key ?? app.key)

  const appOptions = useMemo<AppPickerOption[]>(() => {
    const options: AppPickerOption[] = visibleProcesses.map((p) => ({
      value: p.key,
      label: p.name,
      subtitle: p.executable && p.executable !== p.name ? p.executable : undefined,
      active: p.isOutputActive === true,
      disabled: appTargets.length >= MAX_APP_TARGETS && !selectedKeys.includes(p.key),
    }))
    for (const { app, process } of selectedProcesses) {
      if (process) continue
      options.push({
        value: app.key,
        label: app.name,
        subtitle: audioProcesses.some((p) => p.key === app.key || p.name === app.name)
          ? t('audio.appIdle')
          : t('audio.appNotRunning'),
      })
    }
    return options
  }, [visibleProcesses, selectedProcesses, selectedKeys, appTargets.length, audioProcesses, t])

  const toggleApp = useCallback(
    (key: string) => {
      const selected = selectedProcesses.find(({ app, process }) => (process?.key ?? app.key) === key)
      if (selected) {
        setAppTargets(appTargets.filter((app) => app !== selected.app))
        return
      }
      if (appTargets.length >= MAX_APP_TARGETS) return
      const option = appOptions.find((o) => o.value === key)
      if (!option) return
      setAppTargets([...appTargets, { key, name: option.label }])
    },
    [appTargets, selectedProcesses, appOptions, setAppTargets],
  )

  const appPickerAction = {
    label: showAllApps ? t('audio.appOnlyPlaying') : t('audio.appShowAll'),
    subtitle: showAllApps ? undefined : t('audio.appShowAllHint'),
    onClick: () => setShowAllApps((v) => !v),
  }

  const startPreview = useCallback(() => {
    if (!prefsReady || devicesLoading || capturing) return
    const seq = ++previewSeq.current
    monitorStart(targets)
      .then(() => {
        if (seq !== previewSeq.current) return
        setPreviewOn(true)
        setMonitorError(null)
      })
      .catch((e) => {
        if (seq !== previewSeq.current) return
        setPreviewOn(false)
        setMonitorError({ message: formatCmdError(e), permission: permissionOf(e) !== null })
      })
  }, [prefsReady, devicesLoading, capturing, targets])

  useEffect(() => {
    if (capturing) {
      setPreviewOn(false)
      setMonitorError(null)
      return
    }
    if (!prefsReady || devicesLoading) return
    const timer = window.setTimeout(() => void startPreview(), 200)
    return () => window.clearTimeout(timer)
  }, [startPreview, capturing, prefsReady, devicesLoading])

  useEffect(() => {
    const onFocus = () => void startPreview()
    window.addEventListener('focus', onFocus)
    return () => window.removeEventListener('focus', onFocus)
  }, [startPreview])

  useEffect(() => {
    return () => {
      previewSeq.current += 1
      monitorStop().catch(() => {})
    }
  }, [])

  return (
    <Page title={t('audio.title')}>
      {deviceError && dismissedDeviceError !== deviceError && (
        <Alert
          tone="warn"
          title={t('audio.enumerateFailed')}
          onDismiss={() => setDismissedDeviceError(deviceError)}
          actions={
            <Button size="sm" onClick={onOpenPermissions}>
              {t('audio.viewPermissions')}
            </Button>
          }
        >
          <p className="break-words font-mono text-[0.95em]">{deviceError}</p>
          <p className="mt-1">{t('audio.enumerateHintDefault')}</p>
          <p className="mt-1">{t('audio.enumerateHintPermission')}</p>
        </Alert>
      )}

      {monitorError && (
        <Alert
          tone="warn"
          title={t('audio.previewFailed')}
          onDismiss={() => setMonitorError(null)}
          actions={
            <>
              {monitorError.permission && (
                <Button size="sm" onClick={onOpenPermissions}>
                  {t('audio.viewPermissions')}
                </Button>
              )}
              <Button size="sm" variant="ghost" onClick={() => void startPreview()}>
                {t('common.retry')}
              </Button>
            </>
          }
        >
          <p className="break-words font-mono text-[0.95em]">{monitorError.message}</p>
        </Alert>
      )}

      {audio?.silent && audio.segments === 0 && (
        <SilenceHelp
          audioSource={audioSource}
          appName={isApp && appTargets.length > 0 ? appTargets.map((a) => a.name).join(' + ') : undefined}
          onOpenPermissions={onOpenPermissions}
        />
      )}

      <section className="space-y-2">
        <SectionLabel>{t('audio.sourceSection')}</SectionLabel>
        <ChoiceGroup label={t('audio.sourceSection')}>
          {sourceOptions.map((o) => (
            <ChoiceCard
              key={o.value}
              title={o.label}
              description={o.description}
              selected={audioSource === o.value}
              disabled={capturing}
              onSelect={() => {
                setOpenMenu(null)
                setAudioSource(o.value)
              }}
            />
          ))}
        </ChoiceGroup>
        <p className="px-1 text-[var(--color-ink-2)] text-[0.7rem] leading-relaxed">{t('audio.sourceHint')}</p>
      </section>

      <SettingsGroup title={isApp ? t('audio.appSection') : t('audio.inputDevice')}>
        {isMix ? (
          <>
            <DeviceRow
              icon="waveform"
              title={t('permissions.mic')}
              label={t('audio.micDevice')}
              devices={mixMicDevices}
              value={mixMicDevice}
              source={AudioSourceKind.Mic}
              capturing={capturing}
              previewing={previewing}
              loading={devicesLoading}
              onChange={setMixMicDevice}
              onOpenChange={(open) => setOpenMenu(open ? 'mic' : null)}
              onRefresh={() => void refreshDevices()}
            />
            <DeviceRow
              icon="speaker"
              title={t('audio.sourceLoopbackLabel')}
              label={t('audio.systemDevice')}
              devices={mixSystemDevices}
              value={mixSystemDevice}
              source={AudioSourceKind.Loopback}
              capturing={capturing}
              previewing={previewing}
              loading={devicesLoading}
              onChange={setMixSystemDevice}
              onOpenChange={(open) => setOpenMenu(open ? 'system' : null)}
              onRefresh={() => void refreshDevices()}
            />
          </>
        ) : isApp ? (
          <SettingsRow
            icon="overlay"
            title={t('audio.app')}
            description={
              processError ??
              (processesLoading && audioProcesses.length === 0
                ? t('audio.appLoading')
                : audioProcesses.length === 0
                  ? t('audio.appEmpty')
                  : !showAllApps && visibleProcesses.length === 0
                    ? t('audio.appNoPlaying')
                    : appTargets.length === 0
                      ? t('audio.appPickHint')
                      : t('audio.appHint'))
            }
            control={
              <div className="flex items-center gap-1.5">
                {capturing ? (
                  <LevelMeter active />
                ) : previewing ? (
                  <DeviceLevelMeter device={null} source={AudioSourceKind.App} apps={selectedKeys} active />
                ) : null}
                <AppPicker
                  label={t('audio.app')}
                  placeholder={t('audio.appSelectPlaceholder')}
                  options={appOptions}
                  selected={selectedKeys}
                  onToggle={toggleApp}
                  action={appPickerAction}
                  disabled={capturing || audioProcesses.length === 0}
                  className="w-[15rem]"
                />
                <Button icon="refresh" size="sm" spinning={processesLoading} onClick={() => void refreshProcesses()}>
                  {t('common.refresh')}
                </Button>
              </div>
            }
          />
        ) : (
          <DeviceRow
            icon="waveform"
            title={t('audio.inputDevice')}
            label={t('audio.inputDevice')}
            devices={devices}
            value={device}
            source={audioSource}
            capturing={capturing}
            previewing={previewing}
            loading={devicesLoading}
            onChange={setDevice}
            onOpenChange={(open) => setOpenMenu(open ? 'single' : null)}
            onRefresh={() => void refreshDevices()}
          />
        )}
      </SettingsGroup>

      {capturing && <p className="text-[var(--color-ink-2)] text-[0.7rem]">{t('audio.captureHint')}</p>}
    </Page>
  )
}

function DeviceRow({
  icon,
  title,
  label,
  devices,
  value,
  source,
  capturing,
  previewing,
  loading,
  onChange,
  onOpenChange,
  onRefresh,
}: {
  icon: 'waveform' | 'speaker'
  title: string
  label: string
  devices: InputDevice[]
  value: string
  source: AudioSourceKind
  capturing: boolean
  previewing: boolean
  loading?: boolean
  onChange: (id: string) => void
  onOpenChange: (open: boolean) => void
  onRefresh: () => void
}) {
  const { t } = useI18n()
  const selected = devices.find((d) => d.id === value)
  const description = selected ? describeDevice(selected, t) : undefined

  return (
    <SettingsRow
      icon={icon}
      title={title}
      description={description}
      control={
        <div className="flex items-center gap-1.5">
          <DeviceSelect
            label={label}
            devices={devices}
            value={value}
            source={source}
            meter={
              capturing ? (
                <LevelMeter active />
              ) : (
                <DeviceLevelMeter device={value || null} source={source} active={previewing} />
              )
            }
            streaming={previewing}
            onChange={onChange}
            onOpenChange={onOpenChange}
            disabled={capturing}
            className="w-[15rem]"
          />
          <Button icon="refresh" size="sm" spinning={loading} onClick={onRefresh}>
            {t('common.refresh')}
          </Button>
        </div>
      }
    />
  )
}

function SilenceHelp({
  audioSource,
  appName,
  onOpenPermissions,
}: {
  audioSource: AudioSourceKind
  appName?: string
  onOpenPermissions: () => void
}) {
  const { t } = useI18n()
  const [dismissed, setDismissed] = useState(false)
  const systemAudio = audioSource !== AudioSourceKind.Mic
  const mic = audioSource === AudioSourceKind.Mic || audioSource === AudioSourceKind.Mix

  if (dismissed) return null

  return (
    <Alert
      tone="warn"
      title={t('audio.silenceTitle')}
      onDismiss={() => setDismissed(true)}
      actions={
        <>
          {isMacOS && (
            <Button size="sm" onClick={onOpenPermissions}>
              {t('audio.silenceGoPermissions')}
            </Button>
          )}
          {(isMacOS || isWindows) && (
            <Button
              size="sm"
              variant={isMacOS ? 'ghost' : 'secondary'}
              onClick={() => {
                openSystemSettings().catch(() => {})
              }}
            >
              {t('audio.silenceOpenSettings')}
            </Button>
          )}
        </>
      }
    >
      <ul className="list-inside list-disc space-y-1">
        {systemAudio && (
          <>
            {appName && <li>{t('audio.silenceCheckApp', { app: appName })}</li>}
            <li>{t('audio.silenceCheckPlayer')}</li>
            {isMacOS ? (
              <li>
                {t('audio.silenceCheckPermission')}
                <span className="font-medium">{t('audio.silencePermissionDeniedNote')}</span>
              </li>
            ) : (
              <li>{t(isWindows ? 'audio.silenceCheckDeviceWindows' : 'audio.silenceCheckDeviceLinux')}</li>
            )}
            <li>{t('audio.silenceVirtualDevice')}</li>
          </>
        )}
        {mic && <li>{t('audio.silenceMic')}</li>}
      </ul>
    </Alert>
  )
}
