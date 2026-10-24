import { save } from '@tauri-apps/plugin-dialog'
import { useStore } from '@/main/store'
import { formatCmdError, revealPath, transcriptExport } from '@/shared/ipc'
import { TaskStatus } from '@/shared/types'
import { useTranscriptStream } from '@/shared/useTranscriptStream'
import { Alert } from '@/ui/Alert'
import { Button } from '@/ui/Button'
import { Page } from '@/ui/Page'
import { SegmentList } from '@/ui/SegmentList'

function exportStamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

export function TranscribePage() {
  const { segments, clear, error: streamError } = useTranscriptStream()
  const { status, clearAll, setError } = useStore()
  const capturing = status === TaskStatus.Capturing
  const canExport = segments.some((s) => s.state !== 'partial')

  async function handleClear() {
    try {
      await clearAll()
      clear()
    } catch (e) {
      setError(formatCmdError(e))
    }
  }

  async function handleExport() {
    try {
      const stamp = exportStamp()
      const path = await save({
        defaultPath: `AudioScript-转写-${stamp.replace(/:/g, '-')}.md`,
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
    <Page title="转写">
      {streamError && (
        <Alert tone="error" title="转写内容同步失败">
          {streamError}
        </Alert>
      )}
      <div className="flex items-center gap-2">
        <Button onClick={handleClear} disabled={capturing || segments.length === 0} size="sm">
          清空
        </Button>
        <Button icon="download" size="sm" disabled={!canExport} onClick={() => void handleExport()}>
          导出
        </Button>
        <span className="ml-auto font-mono text-[var(--color-ink-2)] text-[0.7rem]">{segments.length} 段</span>
      </div>

      <div className="max-h-[62vh] overflow-y-auto rounded-[var(--radius-panel)] border border-[var(--color-border-subtle)] bg-[var(--color-surface-0)] p-4">
        {segments.length === 0 ? (
          <p className="py-8 text-center text-[var(--color-ink-2)] text-sm">
            {capturing ? '识别中…' : '点击左上角「开始转写」。'}
          </p>
        ) : (
          <SegmentList segments={segments} />
        )}
      </div>

      {!capturing && segments.length > 0 && (
        <p className="text-[var(--color-ink-2)] text-[0.7rem]">停止转写后内容保留，直至清空或退出应用。</p>
      )}
    </Page>
  )
}
