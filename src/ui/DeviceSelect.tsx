import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from 'react'
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { type MessageKey, useI18n } from '@/shared/i18n'
import type { AudioSourceKind, InputDevice } from '@/shared/ipc'
import { cn } from './cn'
import { Icon } from './Icon'
import { DeviceLevelMeter } from './LevelMeter'

const EDGE = 8
const GAP = 6
const MIN_WIDTH = 288
const MAX_HEIGHT = 320
const ROW_ESTIMATE = 56

interface Placement {
  left: number
  width: number
  maxHeight: number
  top?: number
  bottom?: number
}

function measureMenu(trigger: HTMLElement, count: number): Placement {
  const rect = trigger.getBoundingClientRect()
  const width = Math.min(Math.max(rect.width, MIN_WIDTH), window.innerWidth - EDGE * 2)
  const left = Math.min(Math.max(EDGE, rect.left), window.innerWidth - width - EDGE)
  const below = window.innerHeight - rect.bottom - EDGE - GAP
  const above = rect.top - EDGE - GAP
  const estimated = Math.min(count * ROW_ESTIMATE + 8, MAX_HEIGHT)
  if (below >= estimated || below >= above) {
    return { left, width, maxHeight: Math.min(below, MAX_HEIGHT), top: rect.bottom + GAP }
  }
  return { left, width, maxHeight: Math.min(above, MAX_HEIGHT), bottom: window.innerHeight - rect.top + GAP }
}

export function describeDevice(
  device: InputDevice,
  t: (key: MessageKey, vars?: Record<string, string | number>) => string,
): string {
  const rate =
    device.sampleRate >= 1000
      ? `${(device.sampleRate / 1000).toFixed(1).replace(/\.0$/, '')} kHz`
      : `${device.sampleRate} Hz`
  return (
    t('audio.deviceSpec', { channels: device.channels, rate }) +
    (device.isDefault ? t('common.systemDefaultSuffix') : '')
  )
}

interface Props {
  label: string
  devices: InputDevice[]
  value: string
  source: AudioSourceKind
  meter: ReactNode
  streaming: boolean
  disabled?: boolean
  className?: string
  onChange: (id: string) => void
  onOpenChange: (open: boolean) => void
}

export function DeviceSelect({
  label,
  devices,
  value,
  source,
  meter,
  streaming,
  disabled,
  className,
  onChange,
  onOpenChange,
}: Props) {
  const { t } = useI18n()
  const [open, setOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState(0)
  const [placement, setPlacement] = useState<Placement | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const baseId = useId()
  const menuId = `${baseId}-menu`

  const selected = devices.find((d) => d.id === value)

  const close = useCallback(
    (focus = false) => {
      setOpen(false)
      onOpenChange(false)
      if (focus) triggerRef.current?.focus()
    },
    [onOpenChange],
  )

  const openMenu = useCallback(() => {
    if (disabled || devices.length === 0) return
    setActiveIndex(
      Math.max(
        0,
        devices.findIndex((d) => d.id === value),
      ),
    )
    setOpen(true)
    onOpenChange(true)
  }, [disabled, devices, value, onOpenChange])

  const choose = useCallback(
    (id: string) => {
      onChange(id)
      close(true)
    },
    [onChange, close],
  )

  useLayoutEffect(() => {
    if (!open) {
      setPlacement(null)
      return
    }
    const trigger = triggerRef.current
    if (!trigger) return
    const update = () => setPlacement(measureMenu(trigger, devices.length))
    update()
    const onScroll = (e: Event) => {
      if (menuRef.current?.contains(e.target as Node)) return
      close()
    }
    window.addEventListener('resize', update)
    window.addEventListener('scroll', onScroll, true)
    return () => {
      window.removeEventListener('resize', update)
      window.removeEventListener('scroll', onScroll, true)
    }
  }, [open, devices.length, close])

  useEffect(() => {
    if (!open) return
    menuRef.current?.focus()
    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Node
      if (menuRef.current?.contains(target) || triggerRef.current?.contains(target)) return
      close()
    }
    document.addEventListener('pointerdown', onPointerDown, true)
    return () => document.removeEventListener('pointerdown', onPointerDown, true)
  }, [open, close])

  useEffect(() => {
    if (!open) return
    document.getElementById(`${baseId}-option-${activeIndex}`)?.scrollIntoView({ block: 'nearest' })
  }, [open, activeIndex, baseId])

  useEffect(() => {
    if (!open) return
    const onBlur = () => close()
    window.addEventListener('blur', onBlur)
    return () => window.removeEventListener('blur', onBlur)
  }, [open, close])

  useEffect(() => {
    if (open && (disabled || devices.length === 0)) close()
  }, [disabled, devices.length, open, close])

  const previousSource = useRef(source)
  useEffect(() => {
    if (previousSource.current === source) return
    previousSource.current = source
    if (open) close()
  }, [source, open, close])

  const onMenuKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape') {
      e.preventDefault()
      close(true)
      return
    }
    if (e.key === 'Tab') {
      e.preventDefault()
      close(true)
      return
    }
    if (devices.length === 0) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setActiveIndex((i) => Math.min(i + 1, devices.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActiveIndex((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Home') {
      e.preventDefault()
      setActiveIndex(0)
    } else if (e.key === 'End') {
      e.preventDefault()
      setActiveIndex(devices.length - 1)
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      const device = devices[activeIndex]
      if (device) choose(device.id)
    }
  }

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        disabled={disabled || devices.length === 0}
        onClick={() => (open ? close() : openMenu())}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && open) {
            e.preventDefault()
            close()
            return
          }
          if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return
          e.preventDefault()
          if (open) menuRef.current?.focus()
          else openMenu()
        }}
        className={cn(
          'flex items-center gap-2 rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)]',
          'bg-[var(--color-surface-1)] px-2 py-1.5 text-[var(--color-ink-0)] text-xs transition',
          'disabled:cursor-not-allowed disabled:opacity-40',
          open && 'border-[var(--color-accent)]',
          className,
        )}
      >
        {meter}
        <span className="min-w-0 flex-1 truncate text-left">
          {selected?.name ??
            (devices.length === 0 ? t('audio.deviceSelectNoDevices') : t('audio.deviceSelectPlaceholder'))}
        </span>
        <Icon
          name="chevronDown"
          size={12}
          className={cn('shrink-0 text-[var(--color-ink-2)] transition-transform', open && 'rotate-180')}
        />
      </button>

      {open &&
        placement &&
        createPortal(
          <div
            ref={menuRef}
            id={menuId}
            role="listbox"
            aria-label={label}
            tabIndex={-1}
            aria-activedescendant={devices[activeIndex] ? `${baseId}-option-${activeIndex}` : undefined}
            onKeyDown={onMenuKeyDown}
            style={{
              left: placement.left,
              width: placement.width,
              maxHeight: placement.maxHeight,
              top: placement.top,
              bottom: placement.bottom,
            }}
            className={cn(
              'fixed z-50 overflow-y-auto rounded-[var(--radius-panel)] p-1 outline-none',
              'border border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]',
              'shadow-[0_12px_32px_rgb(0_0_0/0.35)]',
            )}
          >
            {devices.map((device, index) => (
              // biome-ignore lint/a11y/useKeyWithClickEvents: 键盘交互由 listbox 容器的 aria-activedescendant 统一处理
              <div
                id={`${baseId}-option-${index}`}
                key={device.id}
                role="option"
                tabIndex={-1}
                aria-selected={device.id === value}
                onPointerEnter={() => setActiveIndex(index)}
                onClick={() => choose(device.id)}
                className={cn(
                  'flex cursor-pointer items-center gap-2.5 rounded-[var(--radius-ctl)] px-2 py-2 transition-colors',
                  index === activeIndex && 'bg-[var(--color-surface-2)]',
                )}
              >
                <DeviceLevelMeter device={device.id} source={source} active={streaming} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[var(--color-ink-0)] text-xs">{device.name}</div>
                  <div className="mt-0.5 truncate text-[var(--color-ink-2)] text-[0.65rem]">
                    {describeDevice(device, t)}
                  </div>
                </div>
                {device.id === value && <Icon name="check" size={14} className="shrink-0 text-[var(--color-accent)]" />}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  )
}
