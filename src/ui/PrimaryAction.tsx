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
const WHEEL_GLIDE = 2.4
const MASK_OFFSET = 102
const WHEEL_OFFSET = 130
const MASK_START = 102
const WHEEL_START = 130

const PALETTE = [
  '#3186ff 34%',
  '#9378ff 37%',
  '#f96bd6 39%',
  '#fc413d 41%',
  '#fc413d 48%',
  '#ff6b2b 50%',
  '#fec700 52%',
  '#ffdb0f 56%',
  '#88de42 58%',
  '#0ebc5f 61%',
  '#0ebc5f 65%',
  '#2eaab2 70%',
  '#00a9bb 72%',
  '#3186ff 73%',
  '#3186ff 100%',
].join(', ')

const BLUR: CSSProperties = {
  position: 'absolute',
  inset: -12,
  filter: 'blur(2px)',
  transform: 'translateZ(0)',
  backfaceVisibility: 'hidden',
  pointerEvents: 'none',
}

const CLIP: CSSProperties = {
  position: 'absolute',
  inset: 10,
  borderRadius: '999px',
  overflow: 'hidden',
  transform: 'translateZ(0)',
  backfaceVisibility: 'hidden',
}

const WEDGE: CSSProperties = {
  position: 'absolute',
  inset: 0,
  borderRadius: 'inherit',
  scale: '4 1.2',
}

const WHEEL: CSSProperties = {
  position: 'absolute',
  inset: 0,
  top: '50%',
  translate: '0 -50%',
  width: '100%',
  aspectRatio: '1 / 1',
  borderRadius: '50%',
  rotate: `var(--glow-spin, ${WHEEL_START}deg)`,
  background: `conic-gradient(${PALETTE})`,
}

const WEDGE_SOFT: CSSProperties = {
  ...WEDGE,
  maskImage: `conic-gradient(from var(--glow-angle, ${MASK_START}deg), transparent 0 55%, #000 70% 75%, transparent 89%)`,
  WebkitMaskImage: `conic-gradient(from var(--glow-angle, ${MASK_START}deg), transparent 0 55%, #000 70% 75%, transparent 89%)`,
}

const WEDGE_SHARP: CSSProperties = {
  ...WEDGE,
  maskImage: `conic-gradient(from var(--glow-angle, ${MASK_START}deg), transparent 0 62%, #000 82%, transparent 89%)`,
  WebkitMaskImage: `conic-gradient(from var(--glow-angle, ${MASK_START}deg), transparent 0 62%, #000 82%, transparent 89%)`,
}

function useGlowFollow() {
  const ref = useRef<HTMLButtonElement>(null)
  const s = useRef({ a: MASK_START, r: WHEEL_START, target: MASK_START, raf: 0, last: 0 })

  const glide = useCallback((now: number) => {
    const el = ref.current
    if (!el) return
    const st = s.current
    const dt = st.last ? Math.min(0.05, (now - st.last) / 1000) : 1 / 60
    st.last = now

    let da = st.target - st.a
    da -= Math.round(da / 360) * 360
    st.a += da * (1 - Math.exp(-dt * GLIDE))

    let dr = st.target - MASK_OFFSET + WHEEL_OFFSET - st.r
    dr -= Math.round(dr / 360) * 360
    st.r += dr * (1 - Math.exp(-dt * WHEEL_GLIDE))

    el.style.setProperty('--glow-angle', `${st.a.toFixed(2)}deg`)
    el.style.setProperty('--glow-spin', `${st.r.toFixed(2)}deg`)

    st.raf = Math.abs(da) > 0.2 || Math.abs(dr) > 0.2 ? requestAnimationFrame(glide) : 0
    if (!st.raf) st.last = 0
  }, [])

  const onPointerMove = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      const el = ref.current
      if (!el) return
      const st = s.current
      const r = el.getBoundingClientRect()
      const spot = (Math.atan2(e.clientY - r.top - r.height / 2, e.clientX - r.left - r.width / 2) * 180) / Math.PI + 90
      st.target = spot + MASK_OFFSET
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

export function PrimaryAction({ capturing, pending, activeDevice, onToggle }: Props) {
  const { t } = useI18n()
  const { ref, onPointerMove } = useGlowFollow()
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
          'group relative flex w-full items-center justify-center rounded-full px-3 py-2',
          'font-medium text-sm transition disabled:cursor-not-allowed disabled:opacity-50',
          'text-[var(--color-accent-ink)]',
        )}
      >
        <span
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-[inherit] opacity-40 transition-opacity duration-300 group-hover:opacity-100"
        >
          <span style={BLUR}>
            <span style={CLIP}>
              <span style={WEDGE_SOFT}>
                <span style={WHEEL} />
              </span>
            </span>
          </span>
          <span style={BLUR}>
            <span style={CLIP}>
              <span style={WEDGE_SHARP}>
                <span style={WHEEL} />
              </span>
            </span>
          </span>
        </span>
        <span
          aria-hidden="true"
          className={cn(
            'absolute inset-0 rounded-[inherit]',
            capturing ? 'bg-[var(--color-danger)]' : 'bg-[var(--color-success)]',
          )}
        >
          <span className="absolute inset-0 rounded-[inherit] bg-white opacity-0 transition-opacity duration-150 group-hover:opacity-10" />
        </span>
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
