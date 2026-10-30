import type { ReactNode } from 'react'
import { cn } from './cn'

type Tone = 'ok' | 'warn' | 'danger' | 'muted' | 'info' | 'purple'
type Size = 'md' | 'sm'

const TONE: Record<Tone, string> = {
  ok: 'bg-[var(--color-success-bg)] text-[var(--color-success-ink)]',
  warn: 'bg-[var(--color-warn-bg)] text-[var(--color-warn-ink)]',
  danger: 'bg-[var(--color-danger-bg)] text-[var(--color-danger-ink)]',
  muted: 'bg-[var(--color-surface-2)] text-[var(--color-ink-1)]',
  info: 'bg-[var(--color-surface-selected)] text-[var(--color-accent)]',
  purple: 'bg-[var(--color-purple-bg)] text-[var(--color-purple-ink)]',
}

const SIZE: Record<Size, string> = {
  md: 'h-[22px] px-2 text-xs',
  sm: 'h-[18px] px-1.5 text-[0.6875rem]',
}

interface Props {
  children: ReactNode
  tone?: Tone
  size?: Size
}

export function StatusBadge({ children, tone = 'muted', size = 'md' }: Props) {
  return (
    <span className={cn('inline-flex shrink-0 items-center rounded-[4px] font-semibold', SIZE[size], TONE[tone])}>
      {children}
    </span>
  )
}
