import { useI18n } from '@/shared/i18n'
import { cn } from './cn'
import { Icon } from './Icon'

interface Props {
  value: number
  min: number
  max: number
  step?: number
  format?: (v: number) => string
  onChange: (next: number) => void
  onCommit?: (next: number) => void
  defaultValue?: number
  disabled?: boolean
  label: string
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  format = String,
  onChange,
  onCommit,
  defaultValue,
  disabled,
  label,
}: Props) {
  const { t } = useI18n()
  const dirty = defaultValue !== undefined && value !== defaultValue

  return (
    <div className="flex items-center gap-3">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        onPointerUp={onCommit ? (e) => onCommit(Number((e.target as HTMLInputElement).value)) : undefined}
        onKeyUp={onCommit ? (e) => onCommit(Number((e.target as HTMLInputElement).value)) : undefined}
        className="h-1 flex-1 cursor-pointer appearance-none rounded-full bg-[var(--color-surface-2)] accent-[var(--color-accent)] disabled:cursor-not-allowed disabled:opacity-40"
      />
      <span className="w-16 text-right font-mono text-[var(--color-ink-1)] text-xs">{format(value)}</span>
      {defaultValue !== undefined && (
        <button
          type="button"
          aria-label={t('common.resetToDefault', { label })}
          disabled={disabled || !dirty}
          onClick={() => {
            onChange(defaultValue)
            onCommit?.(defaultValue)
          }}
          className={cn(
            'rounded p-0.5 text-[var(--color-ink-2)] transition',
            dirty ? 'hover:bg-[var(--color-surface-2)] hover:text-[var(--color-ink-1)]' : 'invisible',
          )}
        >
          <Icon name="reset" size={13} />
        </button>
      )}
    </div>
  )
}
