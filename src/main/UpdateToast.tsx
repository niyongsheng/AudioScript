import { useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { TaskStatus } from '@/shared/types'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { ProgressBar } from '@/ui/ProgressBar'
import { useStore } from './store'

const IGNORED_UPDATE_KEY = 'audioscript.ignored-update-version'

interface Props {
  hidden: boolean
}

export function UpdateToast({ hidden }: Props) {
  const { t } = useI18n()
  const { status, updateInfo, updatePhase, updateProgress, updateError, installedVersion, installUpdate, restartApp } =
    useStore()
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null)
  const [ignoredVersion, setIgnoredVersion] = useState<string | null>(() => localStorage.getItem(IGNORED_UPDATE_KEY))

  const version = updateInfo?.version
  if (hidden || !updateInfo?.available || !version || version === dismissedVersion || version === ignoredVersion) {
    return null
  }

  const capturing = status === TaskStatus.Capturing
  const percent =
    updateProgress?.total != null
      ? Math.min(100, Math.floor((updateProgress.downloaded / updateProgress.total) * 100))
      : null

  const title = (() => {
    if (updatePhase === 'downloading') {
      return percent == null
        ? t('about.updateDownloading', { version })
        : t('about.updateDownloadingPercent', { version, percent })
    }
    if (updatePhase === 'ready') {
      const done = installedVersion ?? version
      return capturing ? t('about.updateReadyCapturing', { version: done }) : t('about.updateReady', { version: done })
    }
    if (updatePhase === 'error') {
      return t('about.updateError', { error: updateError ?? t('about.unknownError') })
    }
    return t('app.updateAvailable', { version })
  })()

  const close = () => setDismissedVersion(version)
  const ignore = () => {
    localStorage.setItem(IGNORED_UPDATE_KEY, version)
    setIgnoredVersion(version)
  }

  return (
    <div className="fixed end-5 bottom-5 z-[60] w-80 max-w-[calc(100vw-2.5rem)]">
      <Alert
        tone={updatePhase === 'error' ? 'error' : 'info'}
        className="shadow-[0_12px_40px_rgb(0_0_0/0.35)]"
        title={<span className="break-words">{title}</span>}
        onDismiss={close}
        actions={
          <>
            {updatePhase === 'downloading' ? null : updatePhase === 'ready' ? (
              <Button size="sm" variant="primary" icon="refresh" onClick={() => void restartApp()} disabled={capturing}>
                {t('common.restart')}
              </Button>
            ) : (
              <Button
                size="sm"
                variant="primary"
                icon="download"
                onClick={() => void installUpdate()}
                disabled={capturing || updatePhase === 'checking'}
                pending={updatePhase === 'checking'}
              >
                {t('app.updateNow')}
              </Button>
            )}
            <Button size="sm" variant="ghost" onClick={ignore}>
              {t('app.updateIgnore')}
            </Button>
          </>
        }
      >
        {updatePhase === 'downloading' && <ProgressBar value={percent} label={t('about.updateProgress')} />}
      </Alert>
    </div>
  )
}
