import { useI18n } from '@/shared/i18n'
import { cn } from './cn'

interface SelectOption<T extends string> {
  value: T
  label: string
  disabled?: boolean
  unavailableReason?: string
}

interface Props<T extends string> {
  value: T
  options: Array<SelectOption<T>>
  onChange: (next: T) => void
  disabled?: boolean
  label: string
  className?: string
}

export function Select<T extends string>({ value, options, onChange, disabled, label, className }: Props<T>) {
  const { t } = useI18n()
  return (
    <select
      aria-label={label}
      value={value}
      disabled={disabled}
      onChange={(e) => onChange(e.target.value as T)}
      className={cn(
        'rounded-[var(--radius-ctl)] border border-[var(--color-border-subtle)]',
        'bg-[var(--color-surface-1)] px-2 py-1.5 text-[var(--color-ink-0)] text-xs',
        'disabled:cursor-not-allowed disabled:opacity-40',
        className,
      )}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value} disabled={o.disabled || !!o.unavailableReason}>
          {o.label}
          {o.unavailableReason ? t('common.unavailableOption', { reason: o.unavailableReason }) : ''}
        </option>
      ))}
    </select>
  )
}
