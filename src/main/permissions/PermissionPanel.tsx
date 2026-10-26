import { useCallback, useState } from 'react'
import { useStore } from '@/main/store'
import { type MessageKey, useI18n } from '@/shared/i18n'
import {
  formatCmdError,
  MicPermission,
  openPrivacySettings,
  type PrivacyPane,
  permissionProbeSystemAudio,
  permissionRequestMic,
  type SystemAudioProbe,
  SystemAudioVerdict,
} from '@/shared/ipc'
import { isWindows } from '@/shared/platform'
import { TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { LevelBars, LevelMeter } from '@/ui/LevelMeter'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { StatusBadge } from '@/ui/StatusBadge'

const MIC_TONE: Record<MicPermission, 'ok' | 'warn' | 'danger' | 'muted'> = {
  granted: 'ok',
  denied: 'danger',
  restricted: 'danger',
  notDetermined: 'warn',
  notApplicable: 'muted',
  unknown: 'muted',
}

const PROBE_TONE: Record<SystemAudioVerdict, 'ok' | 'warn' | 'danger' | 'muted'> = {
  audible: 'ok',
  silentWhilePlaying: 'danger',
  tapRejected: 'danger',
  noAudioPlaying: 'muted',
  unsupportedOs: 'warn',
  inconclusive: 'muted',
}

const MIC_TEXT: Record<MicPermission, MessageKey> = {
  granted: 'permissions.micGranted',
  denied: 'permissions.micDenied',
  restricted: 'permissions.micRestricted',
  notDetermined: 'permissions.micNotDetermined',
  notApplicable: 'permissions.micNotApplicable',
  unknown: 'permissions.micUnknown',
}

const PROBE_TEXT: Record<SystemAudioVerdict, MessageKey> = {
  audible: 'permissions.verdictAudible',
  silentWhilePlaying: 'permissions.verdictSilentWhilePlaying',
  tapRejected: 'permissions.verdictTapRejected',
  noAudioPlaying: 'permissions.verdictNoAudioPlaying',
  unsupportedOs: 'permissions.verdictUnsupportedOs',
  inconclusive: 'permissions.verdictInconclusive',
}

export function PermissionPanel() {
  const { permissions, refreshPermission, refreshDevices, status } = useStore()
  const { t } = useI18n()
  const capturing = status === TaskStatus.Capturing
  const [probing, setProbing] = useState(false)
  const [requesting, setRequesting] = useState(false)
  const [probe, setProbe] = useState<SystemAudioProbe | null>(null)
  const [note, setNote] = useState<string | null>(null)
  const [failure, setFailure] = useState<string | null>(null)

  const mic = permissions?.microphone ?? MicPermission.Unknown
  const promptable = permissions?.microphonePromptable ?? false

  const micDescription = (value: MicPermission, canPrompt: boolean): string => {
    if (value === MicPermission.Granted) return t('permissions.micDescGranted')
    if (value === MicPermission.Denied || value === MicPermission.Restricted) {
      return t('permissions.micDescDenied')
    }
    if (value === MicPermission.NotDetermined && !canPrompt) {
      return t('permissions.micDescUnbundled')
    }
    if (value === MicPermission.NotDetermined) return t('permissions.micDescNotDetermined')
    return t('permissions.micDescUnknown')
  }

  const formatDbfs = (value: number): string => {
    if (value <= 0) return t('permissions.mute')
    const db = 20 * Math.log10(value)
    if (db <= -60) return '< -60 dBFS'
    return `${Math.round(db)} dBFS`
  }

  const probeDescription = (value: SystemAudioProbe | null): string => {
    if (!value) return t('permissions.probeNoProbe')
    switch (value.verdict) {
      case SystemAudioVerdict.Audible:
        return t('permissions.probeAudible')
      case SystemAudioVerdict.SilentWhilePlaying:
        return t('permissions.probeSilentWhilePlaying')
      case SystemAudioVerdict.TapRejected:
        return t('permissions.probeTapRejected')
      case SystemAudioVerdict.NoAudioPlaying:
        return t('permissions.probeNoAudioPlaying')
      case SystemAudioVerdict.UnsupportedOs:
        return t('permissions.probeUnsupportedOs')
      case SystemAudioVerdict.Inconclusive:
        return value.detail ?? t('permissions.probeInconclusive')
    }
  }

  const runProbe = useCallback(async () => {
    setProbing(true)
    setNote(null)
    setFailure(null)
    try {
      const result = await permissionProbeSystemAudio()
      setProbe(result)
    } catch (e) {
      setFailure(formatCmdError(e))
    } finally {
      setProbing(false)
    }
  }, [])

  const askMic = useCallback(async () => {
    setRequesting(true)
    setNote(null)
    setFailure(null)
    try {
      const result = await permissionRequestMic()
      if (result.timedOut) {
        setNote(t('permissions.micTimeout'))
      }
      await refreshPermission()
      if (result.state === MicPermission.Granted) await refreshDevices()
    } catch (e) {
      setFailure(formatCmdError(e))
    } finally {
      setRequesting(false)
    }
  }, [refreshPermission, refreshDevices, t])

  const open = useCallback(async (pane: PrivacyPane) => {
    try {
      await openPrivacySettings(pane)
    } catch (e) {
      setFailure(formatCmdError(e))
    }
  }, [])

  return (
    <div className="space-y-3">
      {failure && (
        <Alert tone="error" title={t('app.operationFailed')} onDismiss={() => setFailure(null)}>
          <p className="break-words font-mono text-[0.95em]">{failure}</p>
        </Alert>
      )}
      {note && (
        <Alert tone="info" onDismiss={() => setNote(null)}>
          {note}
        </Alert>
      )}

      <SettingsGroup title={t('permissions.title')} footer={t('permissions.footer')}>
        <SettingsRow
          icon="waveform"
          title={t('permissions.mic')}
          description={micDescription(mic, promptable)}
          control={
            <div className="flex items-center gap-1.5">
              <StatusBadge tone={MIC_TONE[mic]}>{t(MIC_TEXT[mic])}</StatusBadge>
              {mic === MicPermission.NotDetermined && promptable && (
                <Button size="sm" pending={requesting} onClick={() => void askMic()}>
                  {t('permissions.requestPermission')}
                </Button>
              )}
              {(mic === MicPermission.Denied ||
                mic === MicPermission.Restricted ||
                (mic === MicPermission.NotDetermined && !promptable)) && (
                <Button size="sm" onClick={() => void open('microphone')}>
                  {t('permissions.openSettings')}
                </Button>
              )}
            </div>
          }
        />

        {!isWindows && (
          <SettingsRow
            icon="speaker"
            title={t('permissions.systemAudio')}
            description={
              <>
                {probing ? t('permissions.probing') : probeDescription(probe)}
                {probing && (
                  <span className="mt-1 flex items-center gap-2">
                    <LevelMeter active />
                    <span className="font-mono">{t('permissions.liveLevel')}</span>
                  </span>
                )}
                {!probing && probe && (
                  <span className="mt-1 flex items-center gap-2">
                    <LevelBars level={probe.peak} />
                    <span className="font-mono">
                      {t('permissions.peakRms', {
                        peak: formatDbfs(probe.peak),
                        rms: formatDbfs(probe.rms),
                      })}
                    </span>
                  </span>
                )}
              </>
            }
            control={
              <div className="flex items-center gap-1.5">
                <StatusBadge tone={probe ? PROBE_TONE[probe.verdict] : 'muted'}>
                  {probe ? t(PROBE_TEXT[probe.verdict]) : t('permissions.notDetected')}
                </StatusBadge>
                {(probe?.verdict === SystemAudioVerdict.SilentWhilePlaying ||
                  probe?.verdict === SystemAudioVerdict.TapRejected) && (
                  <Button size="sm" onClick={() => void open('systemAudio')}>
                    {t('permissions.openSettings')}
                  </Button>
                )}
                <Button size="sm" pending={probing} disabled={capturing || probing} onClick={() => void runProbe()}>
                  {t('permissions.detect')}
                </Button>
              </div>
            }
          />
        )}
      </SettingsGroup>
    </div>
  )
}
