import { useI18n } from '@/shared/i18n'
import { cn } from './cn'

interface Props {
  value: number | null
  label?: string
}

export function ProgressBar({ value, label }: Props) {
  const { t } = useI18n()
  const indeterminate = value == null
  const clamped = indeterminate ? 0 : Math.max(0, Math.min(100, value))

  return (
    <div
      role="progressbar"
      aria-label={label ?? t('common.downloadProgress')}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={indeterminate ? undefined : Math.round(clamped)}
      className="h-1.5 w-36 overflow-hidden rounded-full bg-[var(--color-surface-2)]"
    >
      <div
        className={cn(
          'h-full rounded-full bg-[var(--color-accent)] transition-[width] duration-200',
          indeterminate && 'w-1/3 animate-pulse',
        )}
        style={indeterminate ? undefined : { width: `${clamped}%` }}
      />
    </div>
  )
}
