import { type CSSProperties, useCallback, useEffect, useRef, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { cn } from './cn'
import { Icon } from './Icon'

interface Props {
  capturing: boolean
  pending: boolean
  activeDevice: string | null
  onToggle: () => void
}

const GLIDE = 9

function useSpecularFollow() {
  const ref = useRef<HTMLButtonElement>(null)
  const s = useRef({ cur: 0, target: 0, raf: 0, last: 0 })

  const glide = useCallback((now: number) => {
    const el = ref.current
    if (!el) return
    const st = s.current
    const dt = st.last ? Math.min(0.05, (now - st.last) / 1000) : 1 / 60
    st.last = now
    let diff = st.target - st.cur
    diff -= Math.round(diff / 360) * 360
    st.cur += diff * (1 - Math.exp(-dt * GLIDE))
    el.style.setProperty('--spot', st.cur.toFixed(2))
    st.raf = Math.abs(diff) > 0.2 ? requestAnimationFrame(glide) : 0
    if (!st.raf) st.last = 0
  }, [])

  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      const el = ref.current
      if (!el) return
      const st = s.current
      const r = el.getBoundingClientRect()
      const x = e.clientX - r.left
      const y = e.clientY - r.top
      el.style.setProperty('--px', `${x.toFixed(1)}px`)
      el.style.setProperty('--py', `${y.toFixed(1)}px`)
      st.target = (Math.atan2(y - r.height / 2, x - r.width / 2) * 180) / Math.PI + 90
      if (!st.raf) st.raf = requestAnimationFrame(glide)
    },
    [glide],
  )

  useEffect(() => () => cancelAnimationFrame(s.current.raf), [])

  return { ref, onPointerMove }
}

function formatElapsed(ms: number): string {
  const total = Math.floor(ms / 1000)
  const seconds = String(total % 60).padStart(2, '0')
  const minutes = String(Math.floor(total / 60) % 60).padStart(2, '0')
  const hours = Math.floor(total / 3600)
  return hours > 0 ? `${hours}:${minutes}:${seconds}` : `${minutes}:${seconds}`
}

function useElapsed(capturing: boolean): number {
  const [elapsed, setElapsed] = useState(0)

  useEffect(() => {
    if (!capturing) {
      setElapsed(0)
      return
    }
    const startedAt = Date.now()
    const id = window.setInterval(() => setElapsed(Date.now() - startedAt), 1000)
    return () => window.clearInterval(id)
  }, [capturing])

  return elapsed
}

const PEAK = 14

const RIM: CSSProperties = {
  padding: 1.5,
  background: `conic-gradient(from calc(var(--spot, 0) * 1deg - ${PEAK}deg), transparent 0deg, rgb(255 255 255 / 0.9) ${PEAK}deg, transparent 40deg)`,
  maskImage: 'linear-gradient(#000 0 0), linear-gradient(#000 0 0)',
  maskClip: 'content-box, border-box',
  maskComposite: 'exclude',
}

const POOL: CSSProperties = {
  background: 'radial-gradient(80px circle at var(--px, 50%) var(--py, 50%), rgb(255 255 255 / 0.22), transparent 65%)',
}

export function PrimaryAction({ capturing, pending, activeDevice, onToggle }: Props) {
  const { t } = useI18n()
  const { ref, onPointerMove } = useSpecularFollow()
  const elapsed = useElapsed(capturing)

  const state = pending
    ? { label: capturing ? t('action.stopping') : t('action.starting'), dot: 'bg-[var(--color-warn)] animate-pulse' }
    : capturing
      ? { label: activeDevice ?? t('action.capturing'), dot: 'bg-[var(--color-danger)] animate-pulse' }
      : { label: t('action.ready'), dot: 'bg-[var(--color-success)]' }

  return (
    <div className="space-y-2">
      <button
        ref={ref}
        type="button"
        onClick={onToggle}
        onPointerMove={onPointerMove}
        disabled={pending}
        className={cn(
          'group relative flex w-full items-center justify-center rounded-[var(--radius-ctl)] px-3 py-2',
          'font-medium text-sm transition disabled:cursor-not-allowed disabled:opacity-50',
          'hover:brightness-110',
          capturing ? 'bg-[var(--color-danger)]' : 'bg-[var(--color-success)]',
          'text-[var(--color-accent-ink)]',
        )}
      >
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-[var(--radius-ctl)] opacity-40 transition-opacity duration-200 group-hover:opacity-100"
          style={RIM}
        />
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-[var(--radius-ctl)] opacity-0 transition-opacity duration-200 group-hover:opacity-100"
          style={POOL}
        />
        <span className="relative flex items-center justify-center gap-1.5">
          <Icon name={capturing ? 'stop' : 'play'} size={14} />
          {capturing ? t('action.stop') : t('action.start')}
        </span>
      </button>

      <p className="flex items-center gap-1.5 px-0.5 text-[var(--color-ink-2)] text-[0.7rem]">
        <span aria-hidden="true" className={cn('inline-block size-1.5 shrink-0 rounded-full', state.dot)} />
        {capturing ? (
          <>
            <span className="shrink-0 tabular-nums">{formatElapsed(elapsed)}</span>
            <span aria-hidden="true" className="shrink-0 opacity-60">
              ·
            </span>
            <span className="truncate">{state.label}</span>
          </>
        ) : (
          <span className="truncate">{state.label}</span>
        )}
      </p>
    </div>
  )
}
