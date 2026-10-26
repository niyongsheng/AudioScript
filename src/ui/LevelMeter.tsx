import { useEffect, useRef, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { type AudioSourceKind, type DeviceLevelEvent, EVENT, on } from '@/shared/ipc'
import { cn } from './cn'

const SEGMENTS = [0, 1, 2, 3, 4] as const

const FLOOR_DB = -60
const CEIL_DB = -6

const FALL = 0.75

function levelRatio(value: number): number {
  const db = 20 * Math.log10(Math.max(value, 1e-9))
  return Math.min(1, Math.max(0, (db - FLOOR_DB) / (CEIL_DB - FLOOR_DB)))
}

function Dots({ dots, animated }: { dots: number; animated?: boolean }) {
  const { t } = useI18n()
  return (
    <div className="flex items-center gap-1" role="img" aria-label={t('common.inputLevel')}>
      {SEGMENTS.map((seg) => (
        <span
          key={seg}
          className={cn(
            'size-1.5 rounded-full',
            animated && 'transition-colors duration-150',
            seg < dots ? 'bg-[var(--color-success)]' : 'bg-[var(--color-surface-2)]',
          )}
        />
      ))}
    </div>
  )
}

export function LevelBars({ level }: { level: number }) {
  return <Dots dots={Math.ceil(levelRatio(level) * SEGMENTS.length)} />
}

function usePeakDots<T>(event: string, active: boolean, select: (payload: T) => number | null): number {
  const [dots, setDots] = useState(0)
  const peak = useRef(0)
  const selectRef = useRef(select)

  useEffect(() => {
    selectRef.current = select
  }, [select])

  useEffect(() => {
    peak.current = 0
    setDots(0)
    if (!active) return

    let disposed = false
    let unlisten: (() => void) | undefined

    on<T>(event, (payload) => {
      const rms = selectRef.current(payload)
      if (rms === null) return
      const target = levelRatio(rms)
      peak.current = Math.max(target, peak.current * FALL)
      setDots(Math.ceil(peak.current * SEGMENTS.length))
    }).then((fn) => {
      if (disposed) fn()
      else unlisten = fn
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [active, event])

  return dots
}

export function DeviceLevelMeter({
  device,
  source,
  active,
}: {
  device: string | null
  source: AudioSourceKind
  active: boolean
}) {
  const dots = usePeakDots<DeviceLevelEvent>(EVENT.deviceLevel, active, (e) =>
    e.source === source && (e.device ?? null) === device ? e.rms : null,
  )
  return <Dots dots={dots} animated />
}

export function LevelMeter({ active }: { active: boolean }) {
  const dots = usePeakDots<number>(EVENT.level, active, (rms) => (typeof rms === 'number' ? rms : null))
  return <Dots dots={dots} animated />
}
