import { type CSSProperties, useEffect, useId, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { GLOW_COLORS, GLOW_RING_PALETTE } from '@/shared/glow'
import { useI18n } from '@/shared/i18n'
import { Button } from '@/ui/Button'
import { Icon } from '@/ui/Icon'
import { LogoMark } from './Brand'

const BURST = 140
const RAIN = 48
const GRAVITY = 1500
const AUTO_CLOSE_MS = 5000
const COUNTDOWN_RADIUS = 14
const COUNTDOWN_LENGTH = 2 * Math.PI * COUNTDOWN_RADIUS

const RING = `conic-gradient(${GLOW_RING_PALETTE})`
const TITLE = `linear-gradient(90deg, ${GLOW_COLORS.slice(0, 3).join(', ')})`

const SPARKS = [
  { left: '-4%', top: '8%', delay: '0ms', color: GLOW_COLORS[0] },
  { left: '76%', top: '0%', delay: '520ms', color: GLOW_COLORS[2] },
  { left: '94%', top: '54%', delay: '1040ms', color: GLOW_COLORS[7] },
  { left: '10%', top: '80%', delay: '1560ms', color: GLOW_COLORS[8] },
  { left: '42%', top: '94%', delay: '2080ms', color: GLOW_COLORS[1] },
] as const

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  width: number
  height: number
  rotation: number
  spin: number
  color: string
  life: number
  ttl: number
  round: boolean
  gravity: number
}

function pickColor(): string {
  return GLOW_COLORS[Math.floor(Math.random() * GLOW_COLORS.length)]
}

function makeParticles(): Particle[] {
  const width = window.innerWidth
  const height = window.innerHeight
  const cx = width / 2
  const cy = height / 2 - 40

  const burst: Particle[] = Array.from({ length: BURST }, () => {
    const angle = -Math.PI / 2 + (Math.random() - 0.5) * Math.PI * 1.25
    const speed = 320 + Math.random() * 760
    return {
      x: cx + (Math.random() - 0.5) * 40,
      y: cy + (Math.random() - 0.5) * 24,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed,
      width: 5 + Math.random() * 6,
      height: 8 + Math.random() * 9,
      rotation: Math.random() * Math.PI * 2,
      spin: (Math.random() - 0.5) * 16,
      color: pickColor(),
      life: 0,
      ttl: 2.2 + Math.random() * 1.6,
      round: Math.random() < 0.18,
      gravity: GRAVITY,
    }
  })

  const rain: Particle[] = Array.from({ length: RAIN }, () => ({
    x: Math.random() * width,
    y: -24 - Math.random() * height * 0.3,
    vx: (Math.random() - 0.5) * 120,
    vy: 40 + Math.random() * 140,
    width: 4 + Math.random() * 5,
    height: 7 + Math.random() * 8,
    rotation: Math.random() * Math.PI * 2,
    spin: (Math.random() - 0.5) * 10,
    color: pickColor(),
    life: 0,
    ttl: 3.4 + Math.random() * 1.6,
    round: Math.random() < 0.18,
    gravity: 420,
  }))

  return [...burst, ...rain]
}

function Confetti() {
  const canvasRef = useRef<HTMLCanvasElement>(null)

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || (window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false)) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return

    let raf = 0
    let last = 0

    const resize = () => {
      const dpr = Math.min(2, window.devicePixelRatio || 1)
      const width = Math.floor(window.innerWidth * dpr)
      const height = Math.floor(window.innerHeight * dpr)
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width
        canvas.height = height
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
    resize()
    window.addEventListener('resize', resize)

    const particles = makeParticles()

    const tick = (now: number) => {
      const dt = last ? Math.min(0.05, (now - last) / 1000) : 1 / 60
      last = now
      ctx.clearRect(0, 0, window.innerWidth, window.innerHeight)

      let alive = 0
      for (const p of particles) {
        p.life += dt
        if (p.life >= p.ttl) continue
        alive += 1

        p.vy += p.gravity * dt
        const damp = Math.exp(-dt * 0.9)
        p.vx *= damp
        p.vy *= damp
        p.x += p.vx * dt
        p.y += p.vy * dt
        p.rotation += p.spin * dt

        ctx.save()
        ctx.globalAlpha = Math.min(1, (p.ttl - p.life) / 0.5)
        ctx.translate(p.x, p.y)
        ctx.rotate(p.rotation)
        ctx.fillStyle = p.color
        if (p.round) {
          ctx.beginPath()
          ctx.arc(0, 0, p.width / 2, 0, Math.PI * 2)
          ctx.fill()
        } else {
          ctx.fillRect(-p.width / 2, -p.height / 2, p.width, p.height)
        }
        ctx.restore()
      }

      if (alive > 0) raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)

    return () => {
      cancelAnimationFrame(raf)
      window.removeEventListener('resize', resize)
    }
  }, [])

  return <canvas ref={canvasRef} className="pointer-events-none absolute inset-0 h-full w-full" />
}

interface Props {
  name: string
  version: string
  platform: string
  arch: string
  onClose: () => void
  onContact: () => void
}

export function EasterEgg({ name, version, platform, arch, onClose, onContact }: Props) {
  const { t } = useI18n()
  const titleId = useId()
  const [burst, setBurst] = useState(0)
  const closeRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    closeRef.current?.focus()
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  useEffect(() => {
    const timer = window.setTimeout(onClose, AUTO_CLOSE_MS)
    return () => window.clearTimeout(timer)
  }, [onClose])

  return createPortal(
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      className="fixed inset-0 z-[90] flex animate-[egg-fade_200ms_ease-out] items-center justify-center overflow-hidden bg-black/60 p-6 backdrop-blur-sm"
    >
      <Confetti key={burst} />

      <div className="relative w-full max-w-[400px] animate-[egg-pop_480ms_cubic-bezier(0.34,1.56,0.64,1)] motion-reduce:animate-none">
        <div
          aria-hidden
          className="absolute -inset-3 rounded-[36px] opacity-40 blur-2xl"
          style={{ background: RING }}
        />
        <div className="relative rounded-[30px] p-px shadow-[0_30px_90px_rgb(0_0_0/0.5)]" style={{ background: RING }}>
          <div className="relative rounded-[29px] bg-[var(--color-surface-0)] px-7 pt-8 pb-7 text-center">
            <button
              ref={closeRef}
              type="button"
              onClick={onClose}
              aria-label={t('common.close')}
              className="absolute end-3 top-3 rounded-full p-1.5 text-[var(--color-ink-2)] transition hover:bg-[var(--color-surface-2)] hover:text-[var(--color-ink-0)]"
            >
              <span aria-hidden className="pointer-events-none absolute -inset-1">
                <svg aria-hidden="true" viewBox="0 0 32 32" className="-rotate-90 size-full">
                  <circle
                    cx="16"
                    cy="16"
                    r={COUNTDOWN_RADIUS}
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="1.5"
                    strokeLinecap="round"
                    strokeDasharray={COUNTDOWN_LENGTH}
                    className="animate-[egg-countdown_5s_linear_forwards] text-[var(--color-accent)] opacity-40 motion-reduce:animate-none"
                    style={{ '--egg-countdown-length': COUNTDOWN_LENGTH } as CSSProperties}
                  />
                </svg>
              </span>
              <Icon name="close" size={15} className="relative" />
            </button>

            <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--color-border-subtle)] px-2.5 py-1 font-mono text-[0.62rem] text-[var(--color-ink-2)] uppercase tracking-[0.16em]">
              <Icon name="sparkle" size={11} className="text-[var(--color-accent)]" />
              {t('egg.badge')}
            </span>

            <div className="relative mx-auto mt-5 h-28 w-28">
              <div
                aria-hidden
                className="absolute inset-0 animate-[egg-spin_16s_linear_infinite] rounded-full opacity-60 blur-2xl motion-reduce:animate-none"
                style={{ background: RING }}
              />
              <button
                type="button"
                onClick={() => setBurst((n) => n + 1)}
                aria-label={t('egg.encore')}
                className="absolute inset-3 grid place-items-center rounded-[26px] bg-[var(--color-surface-1)] shadow-[0_16px_36px_rgb(0_0_0/0.35)] ring-1 ring-white/10 transition hover:scale-[1.05] active:scale-95 motion-reduce:transition-none"
              >
                <LogoMark className="h-12 text-[var(--color-ink-0)] [&>svg]:block [&>svg]:h-full [&>svg]:w-auto" />
              </button>
              {SPARKS.map((spark) => (
                <span
                  key={spark.delay}
                  aria-hidden="true"
                  className="pointer-events-none absolute animate-[egg-twinkle_2.6s_ease-in-out_infinite] motion-reduce:hidden"
                  style={{ left: spark.left, top: spark.top, color: spark.color, animationDelay: spark.delay }}
                >
                  <Icon name="sparkle" size={12} />
                </span>
              ))}
            </div>

            <h2
              id={titleId}
              className="mt-5 bg-clip-text font-semibold text-2xl text-transparent tracking-tight"
              style={{ backgroundImage: TITLE }}
            >
              {name}
            </h2>
            <p className="mt-2 text-[var(--color-ink-1)] text-sm leading-relaxed">{t('egg.body')}</p>
            <p className="mt-2 text-[var(--color-ink-2)] text-xs leading-relaxed">{t('egg.offline', { name })}</p>

            <div className="mt-5 flex items-center justify-center gap-2 font-mono text-[0.65rem] text-[var(--color-ink-2)]">
              <span>v{version}</span>
              <span aria-hidden>·</span>
              <span>
                {platform}/{arch}
              </span>
            </div>

            <Button
              variant="primary"
              icon="mail"
              onClick={onContact}
              className="mt-6 w-full justify-center rounded-full"
            >
              {t('egg.contact')}
            </Button>
          </div>
        </div>
      </div>
    </div>,
    document.body,
  )
}
