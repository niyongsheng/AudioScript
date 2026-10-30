import type { KeyboardEvent as ReactKeyboardEvent } from 'react'
import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { cn } from './cn'
import { Icon } from './Icon'

const EDGE = 8
const GAP = 6

const SIZES = {
  compact: { minWidth: 180, maxHeight: 280, row: 34 },
  rich: { minWidth: 220, maxHeight: 320, row: 48 },
} as const

type Variant = keyof typeof SIZES

interface SelectOption<T extends string> {
  value: T
  label: string
  subtitle?: string
  disabled?: boolean
}

interface Props<T extends string> {
  value: T
  options: Array<SelectOption<T>>
  onChange: (next: T) => void
  disabled?: boolean
  label: string
  variant?: Variant
  className?: string
}

interface Placement {
  left: number
  width: number
  maxHeight: number
  top?: number
  bottom?: number
}

function isBlocked<T extends string>(option: SelectOption<T> | undefined): boolean {
  return !!option?.disabled
}

function measureMenu(trigger: HTMLElement, count: number, variant: Variant): Placement {
  const { minWidth, maxHeight, row } = SIZES[variant]
  const rect = trigger.getBoundingClientRect()
  const width = Math.min(Math.max(rect.width, minWidth), window.innerWidth - EDGE * 2)
  const left = Math.min(Math.max(EDGE, rect.left), window.innerWidth - width - EDGE)
  const below = window.innerHeight - rect.bottom - EDGE - GAP
  const above = rect.top - EDGE - GAP
  const estimated = Math.min(count * row + 8, maxHeight)
  if (below >= estimated || below >= above) {
    return { left, width, maxHeight: Math.min(below, maxHeight), top: rect.bottom + GAP }
  }
  return { left, width, maxHeight: Math.min(above, maxHeight), bottom: window.innerHeight - rect.top + GAP }
}

export function Select<T extends string>({
  value,
  options,
  onChange,
  disabled,
  label,
  variant = 'compact',
  className,
}: Props<T>) {
  const [open, setOpen] = useState(false)
  const [activeIndex, setActiveIndex] = useState(0)
  const [placement, setPlacement] = useState<Placement | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)
  const baseId = useId()
  const menuId = `${baseId}-menu`

  const selected = options.find((option) => option.value === value)

  const firstEnabled = useCallback(() => {
    const index = options.findIndex((option) => !isBlocked(option))
    return Math.max(0, index)
  }, [options])

  const close = useCallback((focus = false) => {
    setOpen(false)
    if (focus) triggerRef.current?.focus()
  }, [])

  const openMenu = useCallback(() => {
    if (disabled || options.length === 0) return
    const index = options.findIndex((option) => option.value === value)
    setActiveIndex(index >= 0 && !isBlocked(options[index]) ? index : firstEnabled())
    setOpen(true)
  }, [disabled, options, value, firstEnabled])

  const choose = useCallback(
    (option: SelectOption<T>) => {
      if (isBlocked(option)) return
      onChange(option.value)
      close(true)
    },
    [onChange, close],
  )

  const step = useCallback(
    (direction: 1 | -1) => {
      setActiveIndex((current) => {
        for (let index = current + direction; index >= 0 && index < options.length; index += direction) {
          if (!isBlocked(options[index])) return index
        }
        return current
      })
    },
    [options],
  )

  const jump = useCallback(
    (edge: 'start' | 'end') => {
      const enabled = options.map((_, index) => index).filter((index) => !isBlocked(options[index]))
      if (enabled.length === 0) return
      setActiveIndex(edge === 'start' ? enabled[0] : enabled[enabled.length - 1])
    },
    [options],
  )

  useLayoutEffect(() => {
    if (!open) {
      setPlacement(null)
      return
    }
    const trigger = triggerRef.current
    if (!trigger) return
    const update = () => setPlacement(measureMenu(trigger, options.length, variant))
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
  }, [open, options.length, variant, close])

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
    if (open && (disabled || options.length === 0)) close()
  }, [disabled, options.length, open, close])

  const onMenuKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape' || e.key === 'Tab') {
      e.preventDefault()
      close(true)
      return
    }
    if (options.length === 0) return
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      step(1)
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      step(-1)
    } else if (e.key === 'Home') {
      e.preventDefault()
      jump('start')
    } else if (e.key === 'End') {
      e.preventDefault()
      jump('end')
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault()
      const option = options[activeIndex]
      if (option) choose(option)
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
        disabled={disabled || options.length === 0}
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
          'flex items-center rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)]',
          'bg-[var(--color-surface-1)] px-2 text-start text-[var(--color-ink-0)] text-xs transition',
          variant === 'rich' ? 'min-w-[8.5rem] gap-2 py-1' : 'min-w-[5.5rem] gap-1.5 py-1.5',
          'disabled:cursor-not-allowed disabled:opacity-40',
          open && 'border-[var(--color-accent)]',
          className,
        )}
      >
        <span className={cn('min-w-0 flex-1', variant === 'compact' && 'truncate text-start')}>
          {variant === 'rich' ? (
            <>
              <span className="block truncate">{selected?.label ?? value}</span>
              <span className="mt-0.5 block truncate font-mono text-[0.6rem] text-[var(--color-ink-2)]">
                {selected?.subtitle ?? ''}
              </span>
            </>
          ) : (
            (selected?.label ?? value)
          )}
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
              const blocked = isBlocked(option)
              return (
                // biome-ignore lint/a11y/useKeyWithClickEvents: 键盘交互由 listbox 容器的 aria-activedescendant 统一处理
                <div
                  id={`${baseId}-option-${index}`}
                  key={option.value}
                  role="option"
                  tabIndex={-1}
                  aria-selected={option.value === value}
                  aria-disabled={blocked || undefined}
                  onPointerEnter={() => {
                    if (!blocked) setActiveIndex(index)
                  }}
                  onClick={() => choose(option)}
                  className={cn(
                    'flex items-center rounded-[var(--radius-ctl)] text-xs transition-colors',
                    variant === 'rich' ? 'gap-2.5 px-2 py-2' : 'gap-2 px-2 py-1.5',
                    blocked
                      ? 'cursor-not-allowed text-[var(--color-ink-2)] opacity-60'
                      : 'cursor-pointer text-[var(--color-ink-0)]',
                    !blocked && index === activeIndex && 'bg-[var(--color-surface-2)]',
                  )}
                >
                  {variant === 'rich' ? (
                    <div className="min-w-0 flex-1">
                      <div className="truncate">{option.label}</div>
                      {option.subtitle && (
                        <div className="mt-0.5 whitespace-normal font-mono text-[0.6rem] text-[var(--color-ink-2)] leading-snug">
                          {option.subtitle}
                        </div>
                      )}
                    </div>
                  ) : (
                    <span className="min-w-0 flex-1 truncate">{option.label}</span>
                  )}
                  {option.value === value && (
                    <Icon name="check" size={14} className="shrink-0 text-[var(--color-accent)]" />
                  )}
                </div>
              )
            })}
          </div>,
          document.body,
        )}
    </>
  )
}
