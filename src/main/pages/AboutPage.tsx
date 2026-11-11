import { useCallback, useEffect, useRef, useState } from 'react'
import { EasterEgg } from '@/main/EasterEgg'
import { useStore } from '@/main/store'
import { track } from '@/shared/analytics'
import { useI18n } from '@/shared/i18n'
import { formatCmdError, type ModelsList, modelsList, openContact, openRepo, openSponsor, openUrl } from '@/shared/ipc'
import { DOWNLOAD_URL, SITE_URL } from '@/shared/links'
import { modelTitle } from '@/shared/models'
import { ModelKind, TaskStatus } from '@/shared/types'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { ProgressBar } from '@/ui/ProgressBar'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'

const EGG_TAPS = 7
const EGG_TAP_WINDOW = 1100

const VERSION_COLORS = ['#fc413d', '#ff6b2b', '#fec700', '#88de42', '#00a9bb', '#3186ff', '#9378ff'] as const

function afdianBadge(label: string): string {
  return `https://img.shields.io/badge/${encodeURIComponent(label)}-Afdian-946ce6?style=for-the-badge&logo=github&logoColor=white`
}

export function AboutPage() {
  const {
    info,
    setError,
    status,
    updateInfo,
    updatePhase,
    updateProgress,
    updateError,
    installedVersion,
    checkUpdate,
    installUpdate,
    restartApp,
  } = useStore()
  const { t } = useI18n()
  const [models, setModels] = useState<ModelsList | null>(null)
  const [modelsFailed, setModelsFailed] = useState(false)
  const [licensesOpen, setLicensesOpen] = useState(false)
  const [eggOpen, setEggOpen] = useState(false)
  const [eggTaps, setEggTaps] = useState(0)
  const [versionPulse, setVersionPulse] = useState(0)
  const lastEggTap = useRef(0)
  const closeEgg = useCallback(() => setEggOpen(false), [])
  const run = useCallback(
    (action: () => Promise<unknown>) => {
      void action().catch((e) => setError(formatCmdError(e)))
    },
    [setError],
  )

  function tapVersion() {
    setVersionPulse((n) => n + 1)
    const now = performance.now()
    const taps = now - lastEggTap.current > EGG_TAP_WINDOW ? 1 : eggTaps + 1
    lastEggTap.current = now
    if (taps >= EGG_TAPS) {
      setEggTaps(0)
      track('easter_egg_opened', { version: info?.version ?? 'unknown' })
      setEggOpen(true)
      return
    }
    setEggTaps(taps)
  }

  useEffect(() => {
    let disposed = false
    modelsList()
      .then((m) => {
        if (!disposed) setModels(m)
      })
      .catch(() => {
        if (!disposed) setModelsFailed(true)
      })
    return () => {
      disposed = true
    }
  }, [])

  const capturing = status === TaskStatus.Capturing
  const percent =
    updateProgress?.total != null
      ? Math.min(100, Math.floor((updateProgress.downloaded / updateProgress.total) * 100))
      : null

  const updateDescription = (() => {
    if (updatePhase === 'downloading') {
      const version = updateInfo?.version ?? ''
      return percent == null
        ? t('about.updateDownloading', { version })
        : t('about.updateDownloadingPercent', { version, percent })
    }
    if (updatePhase === 'ready') {
      const version = installedVersion ?? updateInfo?.version ?? ''
      return capturing ? t('about.updateReadyCapturing', { version }) : t('about.updateReady', { version })
    }
    if (updatePhase === 'error') {
      return t('about.updateError', { error: updateError ?? t('about.unknownError') })
    }
    if (updatePhase === 'checking') return t('about.updateChecking')
    if (updateInfo?.available) {
      return capturing
        ? t('about.updateAvailableCapturing')
        : t('about.updateAvailable', { version: updateInfo.version ?? '' })
    }
    if (updateInfo) return t('about.updateLatest', { version: updateInfo.currentVersion })
    return t('about.updateIdle')
  })()

  return (
    <Page title={t('about.title')}>
      <SettingsGroup title={t('about.appGroup')}>
        <SettingsRow
          icon="info"
          title={t('about.version')}
          onClick={tapVersion}
          noHover
          control={
            <Mono>
              <span
                key={versionPulse}
                className={versionPulse > 0 ? 'inline-block animate-[egg-tap_260ms_ease-out]' : 'inline-block'}
                style={versionPulse > 0 ? { color: VERSION_COLORS[versionPulse % VERSION_COLORS.length] } : undefined}
              >
                {info?.version ?? '—'}
              </span>
            </Mono>
          }
        />
        <SettingsRow
          icon="download"
          title={t('about.checkUpdate')}
          description={
            <>
              {updateDescription}
              {updateInfo?.available && updatePhase === 'idle' && updateInfo.notes && (
                <p className="mt-1 line-clamp-3 whitespace-pre-line">{updateInfo.notes}</p>
              )}
            </>
          }
          control={
            updatePhase === 'downloading' ? (
              <div className="flex items-center gap-2">
                <ProgressBar value={percent} label={t('about.updateProgress')} />
                <Mono>{percent == null ? t('about.downloading') : `${percent}%`}</Mono>
              </div>
            ) : updatePhase === 'ready' ? (
              <Button size="sm" variant="primary" icon="refresh" onClick={() => void restartApp()} disabled={capturing}>
                {t('common.restart')}
              </Button>
            ) : updatePhase === 'error' ? (
              <div className="flex items-center gap-1.5">
                <Button size="sm" onClick={() => void checkUpdate()} disabled={!info}>
                  {t('common.retry')}
                </Button>
                <Button size="sm" variant="primary" icon="external" onClick={() => run(() => openUrl(DOWNLOAD_URL))}>
                  {t('about.manualUpdate')}
                </Button>
              </div>
            ) : updateInfo?.available ? (
              <Button
                size="sm"
                variant="primary"
                icon="download"
                onClick={() => void installUpdate()}
                disabled={capturing || updatePhase === 'checking'}
              >
                {t('common.downloadAndInstall')}
              </Button>
            ) : (
              <Button
                size="sm"
                icon="refresh"
                onClick={() => void checkUpdate()}
                disabled={!info || updatePhase === 'checking'}
                pending={updatePhase === 'checking'}
              >
                {t('common.check')}
              </Button>
            )
          }
        />
        <SettingsRow
          icon="chip"
          title={t('about.platform')}
          control={
            <Mono>
              {info?.platform ?? '—'} · {info?.arch ?? '—'}
            </Mono>
          }
        />
        <SettingsRow
          icon="external"
          title={t('about.website')}
          onClick={() => run(() => openUrl(SITE_URL))}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)] rtl:rotate-180" />}
        />
      </SettingsGroup>

      <SettingsGroup title={t('about.support')}>
        <SettingsRow
          icon="heart"
          title={t('about.sponsor')}
          description={t('about.sponsorDescription')}
          onClick={() => run(openSponsor)}
          control={<img src={afdianBadge(t('about.afdian'))} alt={t('about.afdian')} className="h-7 w-auto" />}
        />
      </SettingsGroup>

      <SettingsGroup title={t('about.licensesGroup')}>
        <SettingsRow
          icon="github"
          title={
            <span className="flex items-center gap-2">
              LICENSE
              <Mono>MIT</Mono>
            </span>
          }
          onClick={() => void openRepo()}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)] rtl:rotate-180" />}
        />
      </SettingsGroup>

      <SettingsGroup title={t('about.modelsLicenses')} footer={licensesOpen ? t('about.licenseFooter') : undefined}>
        <SettingsRow
          icon="shield"
          title={t('about.modelLicense')}
          onClick={() => setLicensesOpen((open) => !open)}
          control={
            <Icon
              name="chevron"
              size={16}
              className={`mt-0.5 text-[var(--color-ink-2)] transition-transform ${licensesOpen ? 'rotate-90' : ''}`}
            />
          }
        />
        {licensesOpen &&
          (models ? (
            [...models.asr, ...models.mt].flatMap((m) =>
              m.licenses.map((l) => (
                <SettingsRow
                  key={`${m.id}-${l.component}`}
                  icon={m.kind === ModelKind.Asr ? 'waveform' : 'translate'}
                  title={l.component}
                  description={
                    <>
                      {modelTitle(m, t)} · {l.license}
                      {l.requiresAttribution && t('about.needsAttribution')}
                      {l.attribution && ` · ${l.attribution}`}
                    </>
                  }
                  control={
                    l.licenseUrl ? (
                      <Button
                        size="sm"
                        variant="ghost"
                        icon="external"
                        onClick={() => run(() => openUrl(l.licenseUrl as string))}
                      >
                        {t('about.licenseButton')}
                      </Button>
                    ) : undefined
                  }
                />
              )),
            )
          ) : (
            <SettingsRow title={modelsFailed ? t('about.licenseLoadFailed') : t('common.loading')} />
          ))}
      </SettingsGroup>

      {eggOpen && info && (
        <EasterEgg
          name={info.name}
          version={info.version}
          platform={info.platform}
          arch={info.arch}
          onClose={closeEgg}
          onContact={() =>
            run(async () => {
              await openContact()
              closeEgg()
            })
          }
        />
      )}
    </Page>
  )
}

function Mono({ children }: { children: React.ReactNode }) {
  return <span className="font-mono text-[var(--color-ink-1)] text-xs">{children}</span>
}
