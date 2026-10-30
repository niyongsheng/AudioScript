import { save } from '@tauri-apps/plugin-dialog'
import { useStore } from '@/main/store'
import { useI18n } from '@/shared/i18n'
import { formatCmdError, revealPath, transcriptExport } from '@/shared/ipc'
import { TaskStatus } from '@/shared/types'
import { useTranscriptStream } from '@/shared/useTranscriptStream'
import { Button } from '@/ui/Button'
import { Page } from '@/ui/Page'
import { SegmentList } from '@/ui/SegmentList'

function exportStamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

export function TranscribePage() {
  const { segments, clear } = useTranscriptStream()
  const { status, clearAll, setError } = useStore()
  const { t } = useI18n()
  const capturing = status === TaskStatus.Capturing
  const canExport = segments.some((s) => s.state !== 'partial')

  async function handleClear() {
    await clearAll()
    clear()
  }

  async function handleExport() {
    try {
      const stamp = exportStamp()
      const path = await save({
        defaultPath: t('transcribe.exportFileName', { stamp: stamp.replace(/:/g, '-') }),
        filters: [{ name: 'Markdown', extensions: ['md'] }],
      })
      if (!path) return
      const { path: saved } = await transcriptExport(stamp, path)
      await revealPath(saved)
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  return (
    <Page title={t('transcribe.title')}>
      <div className="flex items-center gap-2">
        <Button onClick={handleClear} disabled={capturing || segments.length === 0} size="sm">
          {t('common.clear')}
        </Button>
        <Button icon="download" size="sm" disabled={!canExport} onClick={() => void handleExport()}>
          {t('common.export')}
        </Button>
        <span className="ms-auto font-mono text-[var(--color-ink-2)] text-[0.7rem]">
          {t('common.segmentCount', { count: segments.length })}
        </span>
      </div>

      <div className="max-h-[62vh] overflow-y-auto rounded-[var(--radius-panel)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-0)] p-4">
        {segments.length === 0 ? (
          <p className="py-8 text-center text-[var(--color-ink-2)] text-sm">
            {capturing ? t('transcribe.emptyCapturing') : t('transcribe.emptyIdle')}
          </p>
        ) : (
          <SegmentList segments={segments} />
        )}
      </div>

      {!capturing && segments.length > 0 && (
        <p className="text-[var(--color-ink-2)] text-[0.7rem]">{t('transcribe.keepHint')}</p>
      )}
    </Page>
  )
}
