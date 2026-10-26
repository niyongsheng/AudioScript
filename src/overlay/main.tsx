import { getCurrentWindow } from '@tauri-apps/api/window'
import { StrictMode, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import '@/shared/theme.css'
import { useI18n } from '@/shared/i18n'
import { EVENT, on, overlayConfigGet } from '@/shared/ipc'
import { initLocale } from '@/shared/locale'
import { type OverlayConfig, OverlayMode } from '@/shared/types'
import { useTranscriptStream } from '@/shared/useTranscriptStream'
import { StreamView } from './StreamView'

const DEFAULT_CONFIG: OverlayConfig = {
  enable: true,
  mode: OverlayMode.Edit,
  alwaysOnTop: true,
  opacity: 60,
  fontSize: 22,
  fontColor: '#ffffff',
  bgColor: '#101018',
  maxLines: 6,
  glow: true,
}

void initLocale()

function OverlayApp() {
  const { segments } = useTranscriptStream()
  const [config, setConfig] = useState<OverlayConfig>(DEFAULT_CONFIG)

  useEffect(() => {
    const unlisten: Array<() => void> = []
    let disposed = false

    let gotBroadcast = false
    on<OverlayConfig>(EVENT.overlayState, (c) => {
      gotBroadcast = true
      setConfig(c)
    }).then((fn) => {
      if (disposed) fn()
      else unlisten.push(fn)
    })

    overlayConfigGet()
      .then((c) => {
        if (!disposed && !gotBroadcast) setConfig(c)
      })
      .catch(() => {})

    return () => {
      disposed = true
      for (const fn of unlisten) fn()
    }
  }, [])

  return (
    <div className="fixed inset-0">
      <StreamView
        segments={segments}
        fontSize={config.fontSize}
        fontColor={config.fontColor}
        opacity={config.opacity}
        bgColor={config.bgColor}
        glow={config.glow}
      />
      {config.mode === OverlayMode.Edit && <EditChrome />}
    </div>
  )
}

function EditChrome() {
  const { t } = useI18n()
  const win = getCurrentWindow()

  const drag = (e: React.MouseEvent) => {
    if (e.button !== 0) return
    e.preventDefault()
    win.startDragging().catch((err) => console.error('拖动悬浮窗失败', err))
  }

  const resize = (e: React.MouseEvent) => {
    if (e.button !== 0) return
    e.preventDefault()
    win.startResizeDragging('SouthEast').catch((err) => console.error('缩放悬浮窗失败', err))
  }

  return (
    <>
      <div className="pointer-events-none absolute inset-0 rounded-[var(--radius-panel)] border border-[var(--color-accent)] border-dashed opacity-70" />

      <button
        type="button"
        aria-label={t('overlayWindow.move')}
        onMouseDown={drag}
        className="absolute inset-x-0 top-0 z-10 flex h-4 cursor-move items-center justify-center border-0 bg-transparent p-0"
      >
        <span className="h-1 w-10 rounded-full bg-[var(--color-accent)] opacity-60" />
      </button>

      <button
        type="button"
        aria-label={t('overlayWindow.resize')}
        onMouseDown={resize}
        className="absolute right-0 bottom-0 z-10 size-5 cursor-nwse-resize border-0 bg-transparent p-0"
      >
        <span className="absolute right-1 bottom-1 h-2.5 w-2.5 border-[var(--color-accent)] border-r-2 border-b-2 opacity-70" />
      </button>
    </>
  )
}

const root = document.getElementById('overlay-root')
if (!root) throw new Error('#overlay-root 未找到')

createRoot(root).render(
  <StrictMode>
    <OverlayApp />
  </StrictMode>,
)
