import { useEffect, useRef, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { EVENT, on } from '@/shared/ipc'
import { cn } from './cn'

const SEGMENTS = [0, 1, 2, 3, 4] as const

const FLOOR_DB = -60
const CEIL_DB = -6

const FALL = 0.75

const HISTORY = 12

function levelRatio(value: number): number {
  const db = 20 * Math.log10(Math.max(value, 1e-9))
  return Math.min(1, Math.max(0, (db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)))
}

function formatDb(ratio: number): string {
  if (ratio <= 0) return '< -60 dBFS'
  return `${Math.round(FLOOR_DB + ratio * (CEIL_DB - FLOOR_DB))} dBFS`
}

function Dots({ dots }: { dots: number }) {
  return (
    <div className="flex items-center gap-1">
      {SEGMENTS.map((seg) => (
        <span
          key={seg}
          className={cn(
            'size-1.5 rounded-full transition-colors duration-150',
            seg < dots ? 'bg-[var(--color-success)]' : 'bg-[var(--color-surface-2)]',
          )}
        />
      ))}
    </div>
  )
}

type Bar = { id: number; level: number }

export function LevelStatus({ active }: { active: boolean }) {
  const { t } = useI18n()
  const nextId = useRef(0)
  const [meter, setMeter] = useState(() => ({
    peak: 0,
    history: Array.from({ length: HISTORY }, (): Bar => ({ id: nextId.current++, level: 0 })),
  }))

  useEffect(() => {
    if (!active) {
      setMeter({
        peak: 0,
        history: Array.from({ length: HISTORY }, (): Bar => ({ id: nextId.current++, level: 0 })),
      })
      return
    }

    let disposed = false
    let unlisten: (() => void) | undefined

    on<number>(EVENT.level, (rms) => {
      const target = levelRatio(rms)
      const bar: Bar = { id: nextId.current++, level: target }
      setMeter((prev) => ({
        peak: Math.max(target, prev.peak * FALL),
        history: [...prev.history.slice(1), bar],
      }))
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [active])

  return (
    <div
      className={cn('flex items-center gap-3', !active && 'opacity-40')}
      role="img"
      aria-label={`${t('common.inputLevel')} ${formatDb(meter.peak)}`}
    >
      <Dots dots={Math.ceil(meter.peak * SEGMENTS.length)} />
      <span className="w-[4.6rem] font-mono text-[var(--color-ink-2)] text-[0.7rem] tabular-nums">
        {formatDb(meter.peak)}
      </span>
      <div className="flex h-4 items-end gap-0.5">
        {meter.history.map((bar) => (
          <span
            key={bar.id}
            className="w-0.5 rounded-full bg-[var(--color-success)] transition-[height,opacity] duration-150 ease-linear"
            style={{ height: `${Math.max(12, bar.level * 100)}%`, opacity: 0.35 + bar.level * 0.65 }}
          />
        ))}
      </div>
    </div>
  )
}
