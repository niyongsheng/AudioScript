import type { ReactNode } from 'react'
import { cn } from './cn'

type Tone = 'ok' | 'warn' | 'danger' | 'muted' | 'info'

const TONE: Record<Tone, string> = {
  ok: 'bg-[var(--color-success-bg)] text-[var(--color-success-ink)]',
  warn: 'bg-[var(--color-warn-bg)] text-[var(--color-warn-ink)]',
  danger: 'bg-[var(--color-danger-bg)] text-[var(--color-danger-ink)]',
  muted: 'bg-[var(--color-surface-2)] text-[var(--color-ink-1)]',
  info: 'bg-[var(--color-surface-selected)] text-[var(--color-accent)]',
}

interface Props {
  children: ReactNode
  tone?: Tone
}

export function StatusBadge({ children, tone = 'muted' }: Props) {
  return (
    <span
      className={cn('inline-flex h-[22px] shrink-0 items-center rounded-[4px] px-2 font-semibold text-xs', TONE[tone])}
    >
      {children}
    </span>
  )
}
