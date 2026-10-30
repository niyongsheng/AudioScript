import { open } from '@tauri-apps/plugin-dialog'
import { useState } from 'react'
import { useStore } from '@/main/store'
import { useModels } from '@/main/useModels'
import { track } from '@/shared/analytics'
import { type MessageKey, useI18n } from '@/shared/i18n'
import {
  formatCmdError,
  type ModelJobStatus,
  ModelPhase,
  type ModelStatus,
  type ModelsList,
  modelCancel,
  modelDirSet,
  modelDownload,
  modelOpenDir,
  modelRemove,
  openUrl,
} from '@/shared/ipc'
import { MODEL_DOWNLOAD_URL } from '@/shared/links'
import { formatBytes, modelLanguageLabel, modelStatusBadge, modelTitle } from '@/shared/models'
import { ModelKind, TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { Page } from '@/ui/Page'
import { ProgressBar } from '@/ui/ProgressBar'
import { SettingsGroup, SettingsRow } from '@/ui/Settings'
import { StatusBadge } from '@/ui/StatusBadge'

const PHASE_KEYS: Record<ModelPhase, MessageKey> = {
  [ModelPhase.Downloading]: 'models.phaseDownloading',
  [ModelPhase.Verifying]: 'models.phaseVerifying',
  [ModelPhase.Extracting]: 'models.phaseExtracting',
  [ModelPhase.Validating]: 'models.phaseValidating',
  [ModelPhase.Done]: 'models.phaseDone',
  [ModelPhase.Cancelled]: 'models.phaseCancelled',
  [ModelPhase.Failed]: 'models.phaseFailed',
}

function InlineModelPackLink({ text, label, onOpen }: { text: string; label: string; onOpen: () => void }) {
  const at = text.indexOf(label)
  const link = (
    <button type="button" className="text-[var(--color-accent)] underline underline-offset-2" onClick={onOpen}>
      {label}
    </button>
  )
  if (at < 0) {
    return (
      <>
        {text} {link}
      </>
    )
  }
  return (
    <>
      {text.slice(0, at)}
      {link}
      {text.slice(at + label.length)}
    </>
  )
}

export function ModelsPage() {
  const { status, setError } = useStore()
  const { t } = useI18n()
  const capturing = status === TaskStatus.Capturing

  const { list, setList, job, loadError, failure, dismissFailure, clearLoadError, refresh } = useModels()
  const [busyId, setBusyId] = useState<string | null>(null)
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null)
  const [confirmResetDir, setConfirmResetDir] = useState(false)
  const [dirBusy, setDirBusy] = useState(false)

  function retry() {
    void refresh()
  }

  async function download(modelId: string) {
    dismissFailure()
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

  async function changeDir() {
    try {
      const picked = await open({
        directory: true,
        multiple: false,
        title: t('models.storagePickTitle'),
        defaultPath: list?.userRoot,
      })
      if (typeof picked !== 'string') return
      setDirBusy(true)
      setList(await modelDirSet(picked))
      track('setting_changed', { key: 'models_dir', value: 'custom' })
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setDirBusy(false)
    }
  }

  async function resetDir() {
    setDirBusy(true)
    try {
      setList(await modelDirSet(null))
      setConfirmResetDir(false)
      track('setting_changed', { key: 'models_dir', value: 'default' })
    } catch (e) {
      setError(formatCmdError(e))
    } finally {
      setDirBusy(false)
    }
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
          onDismiss={clearLoadError}
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
          onDismiss={dismissFailure}
        >
          <p className="break-words">{failure.message ?? t('models.downloadFailedFallback')}</p>
        </Alert>
      )}

      {confirmResetDir && (
        <Alert
          tone="warn"
          title={t('models.storageResetTitle')}
          onDismiss={() => setConfirmResetDir(false)}
          actions={
            <>
              <Button size="sm" variant="danger" disabled={dirBusy} onClick={() => void resetDir()}>
                {t('models.storageResetConfirm')}
              </Button>
              <Button size="sm" onClick={() => setConfirmResetDir(false)}>
                {t('common.cancel')}
              </Button>
            </>
          }
        >
          {t('models.storageResetWarn', { path: list.userRoot ?? '' })}
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
              onRemove={() => void remove(m.id)}
              onCancelRemove={() => setConfirmRemove(null)}
            />
          ))}
        </SettingsGroup>
      ))}

      <SettingsGroup title={t('models.storageGroup')}>
        <SettingsRow
          icon="folder"
          title={t('models.storageLocation')}
          description={
            <span>
              {list.userRoot ? (
                <span className="break-all font-mono text-[0.65rem]">{list.userRoot}</span>
              ) : (
                t('models.noStorage')
              )}
            </span>
          }
          control={
            <div className="flex items-center gap-1.5">
              {list.userRootCustom && (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={dirBusy || job !== null}
                  onClick={() => setConfirmResetDir(true)}
                >
                  {t('models.storageReset')}
                </Button>
              )}
              <Button size="sm" disabled={dirBusy || job !== null} onClick={() => void changeDir()}>
                {t('models.storageChange')}
              </Button>
            </div>
          }
        />
      </SettingsGroup>

      <p className="px-1 text-[var(--color-ink-2)] text-[0.7rem] leading-relaxed">
        {connected ? (
          <>
            <Icon name="wifiOff" size={12} className="mr-1 inline-block align-[-0.125em]" />
            <InlineModelPackLink
              text={t('models.needsNetwork')}
              label={t('models.needsNetworkLink')}
              onOpen={() => void openUrl(MODEL_DOWNLOAD_URL).catch((e) => setError(formatCmdError(e)))}
            />
          </>
        ) : (
          t('models.noStorage')
        )}
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
  onRemove,
  onCancelRemove,
}: RowProps) {
  const { t } = useI18n()
  const activeJob = job && job.modelId === model.id ? job : null
  const licenses = [...new Set(model.licenses.map((l) => l.license))]
  const needsAttribution = model.licenses.some((l) => l.requiresAttribution)
  const canDelete = model.installed && !model.bundled && model.downloadable && !model.selected && !capturing && !job
  const badge = modelStatusBadge(model)
  const languageBadge = modelLanguageLabel(model, t)

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
          {languageBadge && (
            <StatusBadge size="sm" tone="info">
              {languageBadge}
            </StatusBadge>
          )}
          {model.kind === ModelKind.Asr && (
            <StatusBadge size="sm" tone="purple">
              {t(model.streaming ? 'models.streaming' : 'models.nonStreaming')}
            </StatusBadge>
          )}
          <StatusBadge size="sm" tone={badge.tone}>
            {t(badge.key)}
          </StatusBadge>
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
              {(!model.installed || !model.complete) && model.downloadable && (
                <Button
                  size="sm"
                  variant="primary"
                  icon="download"
                  onClick={onDownload}
                  disabled={busy || job !== null}
                >
                  {model.installed ? t('common.redownload') : t('common.download')}
                </Button>
              )}

              {canDelete &&
                (confirmRemove ? (
                  <>
                    <Button size="sm" variant="danger" icon="trash" onClick={onRemove} disabled={busy}>
                      {t('common.confirmDelete')}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={onCancelRemove}>
                      {t('common.cancel')}
                    </Button>
                  </>
                ) : (
                  <Button size="sm" variant="danger" icon="trash" onClick={onRemove} disabled={busy}>
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
