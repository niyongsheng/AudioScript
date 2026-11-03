import { useEffect, useRef, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { EVENT, on } from '@/shared/ipc'
import { cn } from './cn'

const SEGMENTS = [0, 1, 2, 3, 4] as const

const FLOOR_DB = -60
const CEIL_DB = -6

const FALL = 0.75

const HISTORY = 48

const LEVEL_ROWS = 5

const DOT_SIZE = 2
const DOT_PITCH = 4
const STRIP_WIDTH = HISTORY * DOT_PITCH - (DOT_PITCH - DOT_SIZE)
const STRIP_HEIGHT = LEVEL_ROWS * DOT_PITCH - (DOT_PITCH - DOT_SIZE)
const FADE_MS = 150

const EDGE_MASK = {
  maskImage: 'linear-gradient(to right, transparent, #000 12%, #000 88%, transparent)',
  WebkitMaskImage: 'linear-gradient(to right, transparent, #000 12%, #000 88%, transparent)',
} as const

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

const emptyHistory = () => Array<number>(HISTORY).fill(0)

function drawBars(ctx: CanvasRenderingContext2D, levels: readonly number[], activeColor: string, idleColor: string) {
  ctx.clearRect(0, 0, STRIP_WIDTH, STRIP_HEIGHT)
  for (let column = 0; column < HISTORY; column += 1) {
    const fill = (levels[column] ?? 0) * LEVEL_ROWS
    for (let row = 0; row < LEVEL_ROWS; row += 1) {
      const coverage = Math.max(0, Math.min(1, fill - row))
      const top = coverage > 0 && fill - row <= 1
      ctx.beginPath()
      ctx.arc(
        column * DOT_PITCH + DOT_SIZE / 2,
        STRIP_HEIGHT - row * DOT_PITCH - DOT_SIZE / 2,
        DOT_SIZE / 2,
        0,
        Math.PI * 2,
      )
      if (coverage > 0) {
        ctx.globalAlpha = 0.3 + coverage * 0.7
        ctx.fillStyle = activeColor
        ctx.shadowColor = activeColor
        ctx.shadowBlur = top ? 2 + coverage * 3 : 0
      } else {
        ctx.globalAlpha = 0.25
        ctx.fillStyle = idleColor
        ctx.shadowBlur = 0
      }
      ctx.fill()
    }
  }
  ctx.globalAlpha = 1
  ctx.shadowBlur = 0
}

function LevelBars({ history }: { history: number[] }) {
  const ref = useRef<HTMLCanvasElement>(null)
  const drawn = useRef<number[] | null>(null)
  const raf = useRef(0)

  useEffect(() => {
    const canvas = ref.current
    const ctx = canvas?.getContext('2d')
    if (!canvas || !ctx) return

    const dpr = Math.min(2, window.devicePixelRatio || 1)
    if (canvas.width !== STRIP_WIDTH * dpr || canvas.height !== STRIP_HEIGHT * dpr) {
      canvas.width = STRIP_WIDTH * dpr
      canvas.height = STRIP_HEIGHT * dpr
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

    // canvas 读不到 CSS 变量，借 color / outlineColor 的计算值解析主题色
    const style = getComputedStyle(canvas)
    const activeColor = style.color
    const idleColor = style.outlineColor
    const from = drawn.current ?? history
    const startedAt = performance.now()

    const render = (now: number) => {
      const t = Math.min(1, (now - startedAt) / FADE_MS)
      const levels = history.map((value, i) => {
        const start = from[i] ?? value
        return start + (value - start) * t
      })
      drawBars(ctx, levels, activeColor, idleColor)
      drawn.current = levels
      raf.current = t < 1 ? requestAnimationFrame(render) : 0
    }
    raf.current = requestAnimationFrame(render)

    return () => cancelAnimationFrame(raf.current)
  }, [history])

  return (
    <canvas
      ref={ref}
      className="block"
      style={{
        ...EDGE_MASK,
        width: STRIP_WIDTH,
        height: STRIP_HEIGHT,
        color: 'var(--color-success)',
        outlineColor: 'var(--color-surface-2)',
      }}
    />
  )
}

export function LevelStatus({ active }: { active: boolean }) {
  const { t } = useI18n()
  const [meter, setMeter] = useState(() => ({ peak: 0, history: emptyHistory() }))

  useEffect(() => {
    if (!active) {
      setMeter({ peak: 0, history: emptyHistory() })
      return
    }

    let disposed = false
    let unlisten: (() => void) | undefined

    on<number>(EVENT.level, (rms) => {
      const target = levelRatio(rms)
      setMeter((prev) => ({
        peak: Math.max(target, prev.peak * FALL),
        history: [...prev.history.slice(1), target],
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
      <LevelBars history={meter.history} />
    </div>
  )
}
