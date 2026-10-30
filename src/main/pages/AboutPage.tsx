import { useEffect, useState } from 'react'
import { useStore } from '@/main/store'
import { useI18n } from '@/shared/i18n'
import { formatCmdError, type ModelsList, modelsList, openRepo, openSponsor, openUrl } from '@/shared/ipc'
import { SITE_URL } from '@/shared/links'
import { modelTitle } from '@/shared/models'
import { ModelKind, TaskStatus } from '@/shared/types'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { ProgressBar } from '@/ui/ProgressBar'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'

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

  async function openSite() {
    try {
      await openUrl(SITE_URL)
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  async function openSponsorPage() {
    try {
      await openSponsor()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

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
        <SettingsRow icon="info" title={t('about.version')} control={<Mono>{info?.version ?? '—'}</Mono>} />
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
          onClick={() => void openSite()}
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
        />
      </SettingsGroup>

      <SettingsGroup title={t('about.support')}>
        <SettingsRow
          icon="heart"
          title={t('about.sponsor')}
          description={t('about.sponsorDescription')}
          onClick={() => void openSponsorPage()}
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
          control={<Icon name="chevron" size={16} className="mt-0.5 text-[var(--color-ink-2)]" />}
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
                        onClick={() => void openUrl(l.licenseUrl as string).catch((e) => setError(formatCmdError(e)))}
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
    </Page>
  )
}

function Mono({ children }: { children: React.ReactNode }) {
  return <span className="font-mono text-[var(--color-ink-1)] text-xs">{children}</span>
}
