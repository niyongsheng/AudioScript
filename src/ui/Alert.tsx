import type { ReactNode } from 'react'
import { useI18n } from '@/shared/i18n'
import { cn } from './cn'
import { Icon, type IconName } from './Icon'

type Tone = 'error' | 'warn' | 'info'

interface Props {
  tone: Tone
  title?: ReactNode
  children?: ReactNode
  actions?: ReactNode
  onDismiss?: () => void
  className?: string
}

export function Alert({ tone, title, children, actions, onDismiss, className }: Props) {
  const { t } = useI18n()
  const style = TONE[tone]

  return (
    <div
      role={tone === 'error' ? 'alert' : 'status'}
      className={cn(
        'flex items-start gap-2.5 rounded-[var(--radius-panel)] border-s-[3px] px-3 py-2.5',
        style.wrap,
        className,
      )}
    >
      <Icon name={style.icon} size={14} strokeWidth={2} className={cn('mt-0.5 shrink-0', style.iconColor)} />

      <div className="min-w-0 flex-1">
        {title && <p className={cn('font-medium text-xs', style.title)}>{title}</p>}
        {children && <div className={cn('text-xs leading-relaxed', style.body, title && 'mt-0.5')}>{children}</div>}
        {actions && <div className="mt-2 flex flex-wrap items-center gap-1.5">{actions}</div>}
      </div>

      {onDismiss && (
        <button
          type="button"
          onClick={onDismiss}
          aria-label={t('common.close')}
          className={cn(
            '-me-1 -mt-1 flex size-5 shrink-0 items-center justify-center rounded opacity-60 transition hover:opacity-100',
            style.iconColor,
          )}
        >
          <Icon name="close" size={12} strokeWidth={2.25} />
        </button>
      )}
    </div>
  )
}

const TONE: Record<Tone, { wrap: string; icon: IconName; iconColor: string; title: string; body: string }> = {
  error: {
    wrap: 'border-s-[var(--color-danger)] bg-[var(--color-danger-bg)]',
    icon: 'error',
    iconColor: 'text-[var(--color-danger-ink)]',
    title: 'text-[var(--color-danger-ink)]',
    body: 'text-[var(--color-danger-ink)]',
  },
  warn: {
    wrap: 'border-s-[var(--color-warn)] bg-[var(--color-warn-bg)]',
    icon: 'alert',
    iconColor: 'text-[var(--color-warn-ink)]',
    title: 'text-[var(--color-warn-ink)]',
    body: 'text-[var(--color-warn-ink)]',
  },
  info: {
    wrap: 'border-s-[var(--color-accent)] bg-[var(--color-surface-1)]',
    icon: 'info',
    iconColor: 'text-[var(--color-accent)]',
    title: 'text-[var(--color-ink-0)]',
    body: 'text-[var(--color-ink-1)]',
  },
}
