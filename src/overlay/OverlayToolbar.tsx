import { getCurrentWindow } from '@tauri-apps/api/window'
import type { MouseEvent } from 'react'
import { useI18n } from '@/shared/i18n'
import { formatCmdError, type HotkeyInfo } from '@/shared/ipc'
import { OVERLAY_TOOLBAR_HEIGHT, type OverlayConfig, OverlayDisplayMode, OverlayMode } from '@/shared/types'
import { Icon } from '@/ui/Icon'

interface Props {
  config: OverlayConfig
  shortcuts: HotkeyInfo[]
  busy: boolean
  onHide: () => void
  onLock: () => void
  onDisplayMode: (mode: OverlayDisplayMode) => void
  onError: (message: string) => void
}

const BUTTON =
  'flex h-7 shrink-0 items-center justify-center rounded px-1.5 text-[11px] font-medium text-white hover:bg-white/15 focus-visible:outline-2 focus-visible:outline-white disabled:opacity-40'

export function OverlayToolbar({ config, shortcuts, busy, onHide, onLock, onDisplayMode, onError }: Props) {
  const { t } = useI18n()
  const unlockKey = shortcuts.find((key) => key.id === 'overlay.toggleMode' && key.registered)
  const showKey = shortcuts.find((key) => key.id === 'overlay.toggleVisible' && key.registered)
  const recovery = unlockKey
    ? t('overlayWindow.locked', { shortcut: unlockKey.acceleratorPretty })
    : t('overlayWindow.recover')
  const hideLabel = showKey
    ? t('overlayWindow.hideShortcut', { shortcut: showKey.acceleratorPretty })
    : t('overlayWindow.hide')
  const lockLabel = unlockKey
    ? t('overlayWindow.lock', { shortcut: unlockKey.acceleratorPretty })
    : t('overlayWindow.lockUnavailable')

  const drag = (event: MouseEvent) => {
    if (event.button !== 0) return
    event.preventDefault()
    void getCurrentWindow()
      .startDragging()
      .catch((error) => onError(formatCmdError(error)))
  }

  if (config.mode === OverlayMode.Locked) {
    return (
      <div
        className="pointer-events-none flex shrink-0 items-center justify-center gap-2 bg-[#14141c]/85 px-2 text-[11px] text-white/75"
        style={{ height: OVERLAY_TOOLBAR_HEIGHT }}
        title={`${recovery}. ${t('overlayWindow.recover')}`}
      >
        <Icon name="lock" size={13} />
        <span className="truncate">{recovery}</span>
      </div>
    )
  }

  return (
    <div
      className="flex shrink-0 items-center gap-1 border-b border-white/15 bg-[#14141c] px-1.5"
      style={{ height: OVERLAY_TOOLBAR_HEIGHT }}
    >
      <button
        type="button"
        title={t('overlayWindow.move')}
        aria-label={t('overlayWindow.move')}
        onMouseDown={drag}
        className={`${BUTTON} min-w-8 flex-1 cursor-grab gap-1 active:cursor-grabbing`}
      >
        <span aria-hidden="true" className="text-base leading-none text-white/65">
          ⠿
        </span>
        <span className="truncate">{t('overlayWindow.drag')}</span>
      </button>
      <fieldset
        aria-label={t('overlay.displayMode')}
        disabled={busy}
        className="m-0 flex shrink-0 gap-0.5 rounded border border-white/20 p-0.5"
      >
        {[
          { value: OverlayDisplayMode.Source, label: t('overlay.displaySource') },
          { value: OverlayDisplayMode.Bilingual, label: t('overlay.displayBilingual') },
          { value: OverlayDisplayMode.Translation, label: t('overlay.displayTranslation') },
        ].map((option) => (
          <button
            key={option.value}
            type="button"
            aria-pressed={config.displayMode === option.value}
            title={option.label}
            onClick={() => onDisplayMode(option.value)}
            className={`${BUTTON} ${config.displayMode === option.value ? 'bg-white/25' : ''}`}
          >
            {option.label}
          </button>
        ))}
      </fieldset>
      <button
        type="button"
        className={`${BUTTON} w-7`}
        title={lockLabel}
        aria-label={lockLabel}
        disabled={busy || !unlockKey}
        onClick={onLock}
      >
        <Icon name="lock" size={15} />
      </button>
      <button
        type="button"
        className={`${BUTTON} w-7`}
        title={hideLabel}
        aria-label={hideLabel}
        disabled={busy}
        onClick={onHide}
      >
        <Icon name="close" size={18} />
      </button>
    </div>
  )
}

export function ResizeHandle({ onError }: { onError: (message: string) => void }) {
  const { t } = useI18n()
  const resize = (event: MouseEvent) => {
    if (event.button !== 0) return
    event.preventDefault()
    const direction = document.documentElement.dir === 'rtl' ? 'SouthWest' : 'SouthEast'
    void getCurrentWindow()
      .startResizeDragging(direction)
      .catch((error) => onError(formatCmdError(error)))
  }
  return (
    <button
      type="button"
      aria-label={t('overlayWindow.resize')}
      title={t('overlayWindow.resize')}
      onMouseDown={resize}
      className="absolute end-0 bottom-0 z-10 size-5 cursor-nwse-resize border-0 bg-transparent p-0"
    >
      <span className="absolute end-1 bottom-1 h-2.5 w-2.5 border-white/65 border-e-2 border-b-2" />
    </button>
  )
}
