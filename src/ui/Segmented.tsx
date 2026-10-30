import { cn } from './cn'
import { Icon, type IconName } from './Icon'

export interface SegmentedOption<T extends string> {
  value: T
  label: string
  icon: IconName
}

interface Props<T extends string> {
  label: string
  value: T
  options: SegmentedOption<T>[]
  onChange: (value: T) => void
}

export function Segmented<T extends string>({ label, value, options, onChange }: Props<T>) {
  return (
    <fieldset
      aria-label={label}
      className="m-0 inline-flex min-w-0 items-center rounded-full border border-[var(--color-border-subtle)] bg-[var(--color-surface-0)] p-0.5"
    >
      {options.map((option) => {
        const active = option.value === value
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={active}
            onClick={() => onChange(option.value)}
            className={cn(
              'inline-flex items-center gap-1 rounded-full px-2.5 py-1 font-medium text-xs transition-colors',
              active
                ? 'bg-[var(--color-ink-0)] text-[var(--color-surface-0)]'
                : 'text-[var(--color-ink-2)] hover:text-[var(--color-ink-0)]',
            )}
          >
            <Icon name={option.icon} size={12} strokeWidth={2} />
            {option.label}
          </button>
        )
      })}
    </fieldset>
  )
}
