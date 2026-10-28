import { useCallback, useEffect, useState } from 'react'
import { useStore } from '@/main/store'
import { type MessageKey, useI18n } from '@/shared/i18n'
import {
  EVENT,
  formatCmdError,
  type ModelJobStatus,
  ModelPhase,
  type ModelStatus,
  type ModelsList,
  modelCancel,
  modelDownload,
  modelOpenDir,
  modelRemove,
  modelSelect,
  modelsList,
  on,
} from '@/shared/ipc'
import { modelTitle } from '@/shared/models'
import { ModelKind, TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { ProgressBar } from '@/ui/ProgressBar'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { StatusBadge } from '@/ui/StatusBadge'

interface Failure {
  modelId: string
  message: string
}

const PHASE_KEYS: Record<ModelPhase, MessageKey> = {
  [ModelPhase.Downloading]: 'models.phaseDownloading',
  [ModelPhase.Verifying]: 'models.phaseVerifying',
  [ModelPhase.Extracting]: 'models.phaseExtracting',
  [ModelPhase.Validating]: 'models.phaseValidating',
  [ModelPhase.Done]: 'models.phaseDone',
  [ModelPhase.Cancelled]: 'models.phaseCancelled',
  [ModelPhase.Failed]: 'models.phaseFailed',
}

function statusBadgeInfo(model: ModelStatus): {
  tone: 'ok' | 'warn' | 'danger' | 'muted'
  key: MessageKey
} {
  if (model.complete) {
    return model.bundled ? { tone: 'muted', key: 'models.bundled' } : { tone: 'ok', key: 'models.statusDownloaded' }
  }
  if (model.bundled) return { tone: 'danger', key: 'models.statusBundledMissing' }
  if (model.installed) return { tone: 'danger', key: 'models.statusIncomplete' }
  if (model.downloadable) return { tone: 'warn', key: 'models.statusNotInstalled' }
  return { tone: 'muted', key: 'models.statusNotPublished' }
}

export function ModelsPage() {
  const { status, setError } = useStore()
  const { t } = useI18n()
  const capturing = status === TaskStatus.Capturing

  const [list, setList] = useState<ModelsList | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [job, setJob] = useState<ModelJobStatus | null>(null)
  const [failure, setFailure] = useState<Failure | null>(null)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const next = await modelsList()
      setList(next)
      setJob(next.job ?? null)
      setLoadError(null)
    } catch (e) {
      setLoadError(formatCmdError(e))
    }
  }, [])

  function retry() {
    setLoadError(null)
    void refresh()
  }

  useEffect(() => {
    let disposed = false
    void refresh()

    const unlisten = on<ModelJobStatus>(EVENT.modelsProgress, (payload) => {
      if (disposed) return
      const terminal =
        payload.phase === ModelPhase.Done ||
        payload.phase === ModelPhase.Failed ||
        payload.phase === ModelPhase.Cancelled

      if (!terminal) {
        setJob(payload)
        return
      }

      if (payload.phase === ModelPhase.Failed) {
        setFailure({
          modelId: payload.modelId,
          message: payload.error ?? t('models.downloadFailedFallback'),
        })
      }
      setJob(null)
      void refresh()
    })

    return () => {
      disposed = true
      void unlisten.then((fn) => fn())
    }
  }, [refresh, t])

  async function download(modelId: string) {
    setFailure(null)
    setBusyId(modelId)
    try {
      await modelDownload(modelId)
      await refresh()
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusyId(null)
    }
  }

  async function cancel(jobId: string) {
    try {
      await modelCancel(jobId)
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  async function select(modelId: string) {
    setBusyId(modelId)
    try {
      setList(await modelSelect(modelId))
      setConfirmRemove(null)
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusyId(null)
    }
  }

  async function remove(modelId: string) {
    if (confirmRemove !== modelId) {
      setConfirmRemove(modelId)
      return
    }
    setBusyId(modelId)
    setConfirmRemove(null)
    try {
      setList(await modelRemove(modelId))
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setBusyId(null)
    }
  }

  function openModelsDir() {
    void modelOpenDir().catch((e) => setError(formatCmdError(e)))
  }

  if (!list) {
    return (
      <Page title={t('models.title')}>
        {loadError ? (
          <Alert
            tone="error"
            title={t('models.loadFailed')}
            actions={
              <Button size="sm" onClick={retry}>
                {t('common.retry')}
              </Button>
            }
          >
            <p className="break-words font-mono">{loadError}</p>
          </Alert>
        ) : (
          <p className="text-[var(--color-ink-2)] text-sm">{t('common.loading')}</p>
        )}
      </Page>
    )
  }

  const connected = list.userRoot !== undefined
  const brokenSelected = [...list.asr, ...list.mt].find((m) => m.selected && !m.complete)

  return (
    <Page title={t('models.title')}>
      {loadError && (
        <Alert
          tone="error"
          title={t('models.refreshFailed')}
          onDismiss={() => setLoadError(null)}
          actions={
            <Button size="sm" onClick={retry}>
              {t('common.retry')}
            </Button>
          }
        >
          <p className="break-words font-mono">{loadError}</p>
        </Alert>
      )}

      {brokenSelected && (
        <Alert tone="error" title={t('models.brokenSelectedTitle', { title: modelTitle(brokenSelected, t) })}>
          {brokenSelected.bundled ? t('models.brokenBundled') : t('models.brokenDownloaded')}
        </Alert>
      )}

      {capturing && (
        <Alert tone="info" title={t('models.capturingTitle')}>
          {t('models.capturingBody')}
        </Alert>
      )}

      {failure && (
        <Alert
          tone="error"
          title={t('models.downloadFailedTitle', { title: modelTitleById(list, failure.modelId, t) })}
          onDismiss={() => setFailure(null)}
        >
          <p className="break-words">{failure.message}</p>
        </Alert>
      )}

      {[
        { title: t('models.asrGroup'), items: list.asr },
        { title: t('models.mtGroup'), items: list.mt },
      ].map((group) => (
        <SettingsGroup key={group.title} title={group.title}>
          {group.items.map((m) => (
            <ModelRow
              key={m.id}
              model={m}
              job={job}
              capturing={capturing}
              busy={busyId === m.id}
              confirmRemove={confirmRemove === m.id}
              onDownload={() => void download(m.id)}
              onCancel={() => job && void cancel(job.jobId)}
              onSelect={() => void select(m.id)}
              onRemove={() => void remove(m.id)}
              onCancelRemove={() => setConfirmRemove(null)}
            />
          ))}
        </SettingsGroup>
      ))}

      <p className="px-1 text-[var(--color-ink-2)] text-[0.7rem] leading-relaxed">
        {connected ? t('models.needsNetwork') : t('models.noStorage')}
        {connected && (
          <button type="button" className="ml-1 underline underline-offset-2" onClick={openModelsDir}>
            {t('models.openDir')}
          </button>
        )}
      </p>
    </Page>
  )
}

interface RowProps {
  model: ModelStatus
  job: ModelJobStatus | null
  capturing: boolean
  busy: boolean
  confirmRemove: boolean
  onDownload: () => void
  onCancel: () => void
  onSelect: () => void
  onRemove: () => void
  onCancelRemove: () => void
}

function ModelRow({
  model,
  job,
  capturing,
  busy,
  confirmRemove,
  onDownload,
  onCancel,
  onSelect,
  onRemove,
  onCancelRemove,
}: RowProps) {
  const { t } = useI18n()
  const activeJob = job && job.modelId === model.id ? job : null
  const licenses = [...new Set(model.licenses.map((l) => l.license))]
  const needsAttribution = model.licenses.some((l) => l.requiresAttribution)
  const canDelete = model.installed && !model.bundled && model.downloadable && !model.selected && !capturing && !job
  const badge = statusBadgeInfo(model)

  return (
    <SettingsRow
      icon={model.kind === ModelKind.Asr ? 'waveform' : 'translate'}
      title={
        <span className="flex flex-wrap items-center gap-1.5">
          {modelTitle(model, t)}
          {model.selected && (
            <span
              role="img"
              aria-label={t('models.inUse')}
              title={t('models.inUse')}
              className="inline-flex items-center text-[var(--color-accent)]"
            >
              <Icon name="check" size={15} strokeWidth={2.25} />
            </span>
          )}
          {model.multilingual && <StatusBadge tone="info">{t('models.multilingual')}</StatusBadge>}
          {(!model.selected || model.bundled) && <StatusBadge tone={badge.tone}>{t(badge.key)}</StatusBadge>}
        </span>
      }
      description={
        <span>
          {t('models.installedSize', { size: formatBytes(model.installedBytes) })}
          {licenses.length > 0 && ` · ${licenses.join(' / ')}${needsAttribution ? t('models.licenseAttribution') : ''}`}
          {!model.bundled && model.installed && !model.complete && t('models.incomplete')}
          {model.bundled && !model.complete && t('models.bundledMissing')}
          {!model.bundled && !model.downloadable && t('models.notPublished')}
        </span>
      }
      control={
        <div className="flex items-center gap-2">
          {activeJob ? (
            <>
              <div className="flex flex-col items-end gap-1">
                <ProgressBar
                  value={activeJob.percent ?? null}
                  label={t('models.downloadProgress', { title: modelTitle(model, t) })}
                />
                <span className="font-mono text-[0.65rem] text-[var(--color-ink-2)]">
                  {t(PHASE_KEYS[activeJob.phase])}
                  {activeJob.percent != null && ` ${Math.round(activeJob.percent)}%`}
                </span>
              </div>
              <Button size="sm" variant="ghost" onClick={onCancel}>
                {t('common.cancel')}
              </Button>
            </>
          ) : (
            <>
              {model.selected ? null : model.installed && model.complete ? (
                <Button size="sm" onClick={onSelect} disabled={busy}>
                  {t('common.use')}
                </Button>
              ) : model.downloadable ? (
                <Button
                  size="sm"
                  variant="primary"
                  icon="download"
                  onClick={onDownload}
                  disabled={busy || job !== null}
                >
                  {model.installed ? t('common.redownload') : t('common.download')}
                </Button>
              ) : null}

              {canDelete &&
                (confirmRemove ? (
                  <>
                    <Button size="sm" variant="danger" onClick={onRemove} disabled={busy}>
                      {t('common.confirmDelete')}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={onCancelRemove}>
                      {t('common.cancel')}
                    </Button>
                  </>
                ) : (
                  <Button size="sm" variant="ghost" onClick={onRemove} disabled={busy}>
                    {t('common.delete')}
                  </Button>
                ))}
            </>
          )}
        </div>
      }
    />
  )
}

function modelTitleById(list: ModelsList, id: string, t: (key: MessageKey) => string): string {
  const model = [...list.asr, ...list.mt].find((m) => m.id === id)
  return model ? modelTitle(model, t) : id
}

function formatBytes(bytes: number): string {
  if (!bytes) return '—'
  const mb = bytes / (1024 * 1024)
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} GB`
  if (mb >= 10) return `${Math.round(mb)} MB`
  return `${mb.toFixed(1)} MB`
}
