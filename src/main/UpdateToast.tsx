import { useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { useStore } from './store'

interface Props {
  hidden: boolean
  onOpenAbout: () => void
}

export function UpdateToast({ hidden, onOpenAbout }: Props) {
  const { t } = useI18n()
  const { updateInfo, updatePhase } = useStore()
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null)

  const version = updateInfo?.version
  const show = Boolean(
    !hidden &&
      updateInfo?.available &&
      version &&
      version !== dismissedVersion &&
      updatePhase !== 'downloading' &&
      updatePhase !== 'ready',
  )

  if (!show || !version) return null

  return (
    <div className="fixed right-5 bottom-5 z-[60] w-80 max-w-[calc(100vw-2.5rem)]">
      <Alert
        tone="info"
        className="shadow-[0_12px_40px_rgb(0_0_0/0.35)]"
        title={t('app.updateAvailable', { version })}
        actions={
          <Button size="sm" onClick={onOpenAbout}>
            {t('app.updateGoAbout')}
          </Button>
        }
        onDismiss={() => setDismissedVersion(version)}
      />
    </div>
  )
}
