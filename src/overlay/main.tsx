import { StrictMode, useRef, useState } from 'react'
import { createRoot } from 'react-dom/client'
import '@/shared/theme.css'
import { syncTrackingFromPrefs } from '@/shared/analytics'
import {
  EVENT,
  formatCmdError,
  type HotkeyInfo,
  overlayConfigGet,
  overlayConfigPatch,
  overlayHide,
  overlaySetMode,
  sessionStatus,
  shortcutsGet,
} from '@/shared/ipc'
import { initLocale } from '@/shared/locale'
import { type OverlayConfig, OverlayDisplayMode, OverlayMode, TaskStatus } from '@/shared/types'
import { useBroadcastSnapshot } from '@/shared/useBroadcastSnapshot'
import { useTranscriptStream } from '@/shared/useTranscriptStream'
import { FloralBorder, useEggBloom } from './FloralBorder'
import { FlowingBorder } from './FlowingBorder'
import { OverlayToolbar, ResizeHandle } from './OverlayToolbar'
import { StreamView } from './StreamView'

const DEFAULT_CONFIG: OverlayConfig = {
  enable: true,
  mode: OverlayMode.Edit,
  displayMode: OverlayDisplayMode.Bilingual,
  alwaysOnTop: true,
  opacity: 60,
  fontSize: 22,
  fontColor: '#ffffff',
  bgColor: '#101018',
  maxLines: 6,
  glow: false,
  glowSpeed: 0,
  glowWidth: 2,
}

void initLocale()
void syncTrackingFromPrefs()

function useCapturing(): boolean {
  const [capturing, setCapturing] = useState(false)

  useBroadcastSnapshot<TaskStatus>(
    EVENT.taskState,
    () => sessionStatus().then((info) => info.status),
    (status) => setCapturing(status === TaskStatus.Capturing),
  )

  return capturing
}

function OverlayApp() {
  const { segments } = useTranscriptStream()
  const capturing = useCapturing()
  const [config, setConfig] = useState<OverlayConfig>(DEFAULT_CONFIG)
  const [shortcuts, setShortcuts] = useState<HotkeyInfo[]>([])
  const [busy, setBusy] = useState(false)
  const pending = useRef(false)
  const [error, setError] = useState<string | null>(null)
  const { bloom, finish } = useEggBloom()

  useBroadcastSnapshot<OverlayConfig>(EVENT.overlayState, overlayConfigGet, setConfig)

  useBroadcastSnapshot<HotkeyInfo[]>(EVENT.overlayShortcuts, shortcutsGet, setShortcuts)

  async function perform(action: () => Promise<unknown>) {
    if (pending.current) return
    pending.current = true
    setBusy(true)
    setError(null)
    try {
      await action()
    } catch (err) {
      setError(formatCmdError(err))
    } finally {
      pending.current = false
      setBusy(false)
    }
  }

  const panelStyle = bloom
    ? { left: bloom.margin, top: bloom.margin, width: bloom.width, height: bloom.height }
    : { inset: 0 }

  return (
    <div className="fixed inset-0">
      <div className="absolute" style={panelStyle}>
        <div className="flex h-full flex-col overflow-hidden rounded-[var(--radius-panel)]">
          {!bloom && (
            <OverlayToolbar
              config={config}
              shortcuts={shortcuts}
              busy={busy}
              onHide={() => void perform(overlayHide)}
              onLock={() => void perform(() => overlaySetMode(OverlayMode.Locked))}
              onDisplayMode={(displayMode) => void perform(() => overlayConfigPatch({ displayMode }))}
              onError={setError}
            />
          )}
          {error && !bloom && (
            <div role="alert" className="shrink-0 bg-red-950 px-3 py-1 text-xs text-white">
              {error}
            </div>
          )}
          <div className="min-h-0 flex-1">
            <StreamView
              segments={segments}
              displayMode={config.displayMode}
              fontSize={config.fontSize}
              fontColor={config.fontColor}
              opacity={config.opacity}
              bgColor={config.bgColor}
            />
          </div>
        </div>
        {config.glow && !bloom && (
          <FlowingBorder streaming={capturing} speed={config.glowSpeed} width={config.glowWidth} />
        )}
      </div>
      {bloom && <FloralBorder bloom={bloom} bgColor={config.bgColor} onDone={() => finish(bloom.id)} />}
      {config.mode === OverlayMode.Edit && !bloom && <ResizeHandle onError={setError} />}
    </div>
  )
}

const root = document.getElementById('overlay-root')
if (!root) throw new Error('#overlay-root 未找到')

createRoot(root).render(
  <StrictMode>
    <OverlayApp />
  </StrictMode>,
)
