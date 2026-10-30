import type { ReactNode } from 'react'
import { cn } from './cn'
import { Icon, type IconName } from './Icon'
import { TitleBarSpacer } from './TitleBarSpacer'

const NAV_ICON = { size: 18, strokeWidth: 1.75 } as const

export interface SidebarItem<Id extends string> {
  id: Id
  label: string
  icon: IconName
}

interface Props<Id extends string> {
  items: Array<SidebarItem<Id>>
  current: Id
  onNavigate: (id: Id) => void
  brand?: ReactNode
  header: ReactNode
}

export function Sidebar<Id extends string>({ items, current, onNavigate, brand, header }: Props<Id>) {
  return (
    <nav className="flex w-52 shrink-0 flex-col bg-[var(--color-surface-sidebar)]">
      <TitleBarSpacer />

      {brand && <div className="px-3 py-4">{brand}</div>}

      <div className="px-3 pb-2">{header}</div>

      <ul className="flex-1 space-y-0.5 overflow-y-auto px-2 py-1">
        {items.map((item) => {
          const active = item.id === current
          return (
            <li key={item.id}>
              <button
                type="button"
                onClick={() => onNavigate(item.id)}
                aria-current={active ? 'page' : undefined}
                className={cn(
                  'flex w-full items-center gap-2.5 rounded-[var(--radius-ctl)] px-2.5 py-1.5 text-start text-[0.9375rem] transition',
                  active
                    ? 'bg-[var(--color-accent)] font-semibold text-[var(--color-accent-ink)]'
                    : 'font-medium text-[var(--color-ink-1)] hover:bg-[var(--color-surface-2)]',
                )}
              >
                <Icon name={item.icon} size={NAV_ICON.size} strokeWidth={NAV_ICON.strokeWidth} />
                {item.label}
              </button>
            </li>
          )
        })}
      </ul>
    </nav>
  )
}
