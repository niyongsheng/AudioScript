import { cn } from './cn'

interface Props {
  checked: boolean
  onChange: (next: boolean) => void
  disabled?: boolean
  label: string
}

export function Switch({ checked, onChange, disabled, label }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        'relative inline-flex h-6 w-10 shrink-0 items-center rounded-full transition',
        'disabled:cursor-not-allowed disabled:opacity-40',
        checked ? 'bg-[var(--color-accent)]' : 'bg-[var(--color-surface-2)]',
      )}
    >
      <span
        className={cn(
          'inline-block size-5 rounded-full bg-white shadow-[var(--shadow-ctl)] transition-transform',
          checked ? 'translate-x-[1.125rem] rtl:-translate-x-[1.125rem]' : 'translate-x-0.5 rtl:-translate-x-0.5',
        )}
      />
    </button>
  )
}
