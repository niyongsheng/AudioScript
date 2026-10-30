import type { ReactNode } from 'react'
import { cn } from './cn'
import { Icon, type IconName } from './Icon'

type Variant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'cta'
type Size = 'md' | 'sm'

interface Props {
  children: ReactNode
  onClick?: () => void
  variant?: Variant
  size?: Size
  icon?: IconName
  disabled?: boolean
  pending?: boolean
  className?: string
}

export function Button({
  children,
  onClick,
  variant = 'secondary',
  size = 'md',
  icon,
  disabled,
  pending,
  className,
}: Props) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled || pending}
      className={cn(
        'inline-flex items-center gap-1.5 rounded-[var(--radius-ctl)] font-medium transition',
        'disabled:cursor-not-allowed disabled:opacity-40',
        size === 'md' ? 'px-3.5 py-2 text-sm' : 'px-2.5 py-1.5 text-xs',
        VARIANT[variant],
        className,
      )}
    >
      {icon && (
        <Icon
          name={icon}
          size={size === 'md' ? 14 : 12}
          className={cn(pending && icon === 'refresh' && 'animate-spin')}
        />
      )}
      {children}
      {pending && <span className="ms-0.5 animate-pulse">…</span>}
    </button>
  )
}

const VARIANT: Record<Variant, string> = {
  primary: 'bg-[var(--color-accent)] text-[var(--color-accent-ink)] hover:bg-[var(--color-accent-dim)]',
  secondary: 'border border-[var(--color-border-subtle)] text-[var(--color-ink-1)] hover:bg-[var(--color-surface-2)]',
  danger: 'bg-[var(--color-danger)] text-[var(--color-accent-ink)] hover:brightness-110',
  ghost: 'text-[var(--color-ink-1)] hover:bg-[var(--color-surface-2)]',
  cta: 'bg-[var(--color-success)] text-[var(--color-accent-ink)] hover:brightness-110',
}
