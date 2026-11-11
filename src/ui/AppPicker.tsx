import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { cn } from './cn'
import { Icon } from './Icon'
import { PlayingBars } from './LevelMeter'

const EDGE = 8
const GAP = 6
const MIN_WIDTH = 300
const MAX_HEIGHT = 340
const ROW_ESTIMATE = 48

export interface AppPickerOption {
  value: string
  label: string
  subtitle?: string
  active?: boolean
  disabled?: boolean
}

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
  const estimated = Math.min(count * ROW_ESTIMATE + 56, MAX_HEIGHT)
  if (below >= estimated || below >= above) {
    return { left, width, maxHeight: Math.min(below, MAX_HEIGHT), top: rect.bottom + GAP }
  }
  return {
    left,
    width,
    maxHeight: Math.min(above, MAX_HEIGHT),
    bottom: window.innerHeight - rect.top + GAP,
  }
}

interface Props {
  label: string
  placeholder: string
  options: AppPickerOption[]
  selected: string[]
  onToggle: (value: string) => void
  action: { label: string; subtitle?: string; onClick: () => void }
  disabled?: boolean
  className?: string
}

export function AppPicker({ label, placeholder, options, selected, onToggle, action, disabled, className }: Props) {
  const [open, setOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState(0)
  const [placement, setPlacement] = useState<Placement | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const baseId = useId()
  const menuId = `${baseId}-menu`

  const selectedOptions = options.filter((option) => selected.includes(option.value))
  const anyActive = selectedOptions.some((option) => option.active)

  const close = useCallback((focus = false) => {
    setOpen(false)
    if (focus) triggerRef.current?.focus()
  }, [])

  const openMenu = useCallback(() => {
    if (disabled) return
    setActiveIndex(
      Math.max(
        0,
        options.findIndex((option) => !option.disabled),
      ),
    )
    setOpen(true)
  }, [disabled, options])

  useLayoutEffect(() => {
    if (!open) {
      setPlacement(null)
      return
    }
    const trigger = triggerRef.current
    if (!trigger) return
    const update = () => setPlacement(measureMenu(trigger, options.length + 1))
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
  }, [open, options.length, close])

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
    if (open && disabled) close()
  }, [open, disabled, close])

  useEffect(() => {
    if (activeIndex >= options.length) setActiveIndex(Math.max(0, options.length - 1))
  }, [activeIndex, options.length])

  const step = useCallback(
    (direction: 1 | -1) => {
      setActiveIndex((current) => {
        for (let index = current + direction; index >= 0 && index < options.length; index += direction) {
          if (!options[index].disabled) return index
        }
        return current
      })
    },
    [options],
  )

  const onMenuKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape' || e.key === 'Tab') {
      e.preventDefault()
      close(true)
      return
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      step(1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      step(-1)
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      const option = options[activeIndex]
      if (option && !option.disabled) onToggle(option.value)
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
        disabled={disabled}
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
          'flex min-w-[8.5rem] items-center gap-2 rounded-[var(--radius-ctl)] border px-2 py-1 text-start text-[var(--color-ink-0)] text-xs transition',
          'border-[var(--color-border-subtle)] bg-[var(--color-surface-1)]',
          'disabled:cursor-not-allowed disabled:opacity-40',
          open && 'border-[var(--color-accent)]',
          className,
        )}
      >
        <span className="min-w-0 flex-1">
          <span className="flex items-center gap-1.5">
            {anyActive && <PlayingBars className="shrink-0" />}
            <span className="truncate">
              {selectedOptions.length > 0 ? selectedOptions.map((option) => option.label).join(' + ') : placeholder}
            </span>
          </span>
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
            aria-multiselectable="true"
            tabIndex={-1}
            aria-activedescendant={options[activeIndex] ? `${baseId}-option-${activeIndex}` : undefined}
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
            {options.map((option, index) => {
              const checked = selected.includes(option.value)
              return (
                // biome-ignore lint/a11y/useKeyWithClickEvents: 键盘交互由 listbox 容器统一处理
                <div
                  id={`${baseId}-option-${index}`}
                  key={option.value}
                  role="option"
                  tabIndex={-1}
                  aria-selected={checked}
                  aria-disabled={option.disabled || undefined}
                  onPointerEnter={() => {
                    if (!option.disabled) setActiveIndex(index)
                  }}
                  onClick={() => {
                    if (!option.disabled) onToggle(option.value)
                  }}
                  className={cn(
                    'flex items-center gap-2.5 rounded-[var(--radius-ctl)] px-2 py-2 text-xs transition-colors',
                    option.disabled
                      ? 'cursor-not-allowed text-[var(--color-ink-2)] opacity-60'
                      : 'cursor-pointer text-[var(--color-ink-0)]',
                    !option.disabled && index === activeIndex && 'bg-[var(--color-surface-2)]',
                  )}
                >
                  {option.active && <PlayingBars className="shrink-0" />}
                  <div className="min-w-0 flex-1">
                    <div className="truncate">{option.label}</div>
                    {option.subtitle && (
                      <div className="mt-0.5 whitespace-normal font-mono text-[0.6rem] text-[var(--color-ink-2)] leading-snug">
                        {option.subtitle}
                      </div>
                    )}
                  </div>
                  {checked && <Icon name="check" size={14} className="shrink-0 text-[var(--color-accent)]" />}
                </div>
              )
            })}
            <button
              type="button"
              onClick={action.onClick}
              className={cn(
                'mt-1 w-full rounded-[var(--radius-ctl)] border-[var(--color-border-subtle)] border-t px-2 py-2 text-start text-xs transition-colors',
                'text-[var(--color-ink-1)] hover:bg-[var(--color-surface-2)]',
              )}
            >
              <span className="block">{action.label}</span>
              {action.subtitle && (
                <span className="mt-0.5 block font-mono text-[0.6rem] text-[var(--color-ink-2)] leading-snug">
                  {action.subtitle}
                </span>
              )}
            </button>
          </div>,
          document.body,
        )}
    </>
  )
}
