import type { ReactNode } from 'react'
import { cn } from './cn'
import { Icon, type IconName } from './Icon'

interface GroupProps {
  children: ReactNode
  title?: string
  footer?: ReactNode
}

export function SectionLabel({ children }: { children: ReactNode }) {
  return <h3 className="px-1 font-medium text-[var(--color-ink-2)] text-xs">{children}</h3>
}

export function SettingsGroup({ children, title, footer }: GroupProps) {
  return (
    <section className="space-y-2">
      {title && <SectionLabel>{title}</SectionLabel>}
      <div
        className={cn(
          'overflow-hidden rounded-[var(--radius-card)] bg-[var(--color-surface-1)]',
          '[&>*+*]:relative',
          '[&>*+*]:before:absolute [&>*+*]:before:inset-x-4 [&>*+*]:before:top-0 [&>*+*]:before:h-px',
          '[&>*+*]:before:bg-[var(--color-border-subtle)] [&>*+*]:before:opacity-60 [&>*+*]:before:content-[""]',
        )}
      >
        {children}
      </div>
      {footer && <p className="px-1 text-[var(--color-ink-2)] text-[0.7rem] leading-relaxed">{footer}</p>}
    </section>
  )
}

interface RowProps {
  title: ReactNode
  icon?: IconName
  description?: ReactNode
  control?: ReactNode
  onClick?: () => void
  unavailableReason?: string
}

export function SettingsRow({ title, icon, description, control, onClick, unavailableReason }: RowProps) {
  const content = (
    <>
      {icon && <Icon name={icon} size={17} className="mt-0.5 shrink-0 text-[var(--color-ink-1)]" />}
      <div className={cn('min-w-0 flex-1', !icon && 'ps-0')}>
        <div className="text-[var(--color-ink-0)] text-sm">{title}</div>
        {(description || unavailableReason) && (
          <div className="mt-0.5 text-[var(--color-ink-2)] text-xs leading-relaxed">
            {description}
            {unavailableReason && <span className="ms-1 italic">{unavailableReason}</span>}
          </div>
        )}
      </div>
      {control && <div className="shrink-0 ps-3">{control}</div>}
    </>
  )

  const base = cn('flex items-start gap-3 px-4 py-3', unavailableReason && 'opacity-50')

  if (!onClick) return <div className={base}>{content}</div>

  return (
    <button
      type="button"
      onClick={onClick}
      disabled={!!unavailableReason}
      className={cn(base, 'w-full text-start transition', !unavailableReason && 'hover:bg-[var(--color-surface-2)]')}
    >
      {content}
    </button>
  )
}
