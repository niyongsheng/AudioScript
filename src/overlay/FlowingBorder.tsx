import { type CSSProperties, useEffect, useRef } from 'react'
import { GLOW_RING_PALETTE } from '@/shared/glow'
import { MAX_GLOW_SPEED } from '@/shared/types'

const MAX_FLOW_DEG_PER_SEC = 120
const BORDER_WIDTH = 2
const HALO_WIDTH = 3
const HALO_BLUR = 4

const MASK_IMAGE = 'linear-gradient(#000 0 0), linear-gradient(#000 0 0)'

function ringStyle(thickness: number, blur: number): CSSProperties {
  return {
    position: 'absolute',
    inset: 0,
    borderRadius: 'inherit',
    padding: thickness,
    background: `conic-gradient(from var(--flow-angle, 0deg), ${GLOW_RING_PALETTE})`,
    maskImage: MASK_IMAGE,
    maskClip: 'content-box, border-box',
    maskComposite: 'exclude',
    WebkitMaskImage: MASK_IMAGE,
    WebkitMaskClip: 'content-box, border-box',
    WebkitMaskComposite: 'xor',
    filter: blur > 0 ? `blur(${blur}px)` : undefined,
    transition: 'opacity 500ms ease',
  }
}

const HALO_STYLE = ringStyle(HALO_WIDTH, HALO_BLUR)
const RING_STYLE = ringStyle(BORDER_WIDTH, 0)

export function FlowingBorder({ streaming, speed }: { streaming: boolean; speed: number }) {
  const frame = useRef<HTMLDivElement>(null)
  const angle = useRef(0)
  const animating = streaming && speed > 0
  const degPerSec = (speed / MAX_GLOW_SPEED) * MAX_FLOW_DEG_PER_SEC

  useEffect(() => {
    if (!animating) return
    let raf = 0
    let last = 0

    const tick = (now: number) => {
      const dt = last ? Math.min(0.05, (now - last) / 1000) : 0
      last = now
      angle.current = (angle.current + degPerSec * dt) % 360
      frame.current?.style.setProperty('--flow-angle', `${angle.current.toFixed(2)}deg`)
      raf = requestAnimationFrame(tick)
    }

    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [animating, degPerSec])

  const lit = streaming || speed <= 0

  return (
    <div ref={frame} aria-hidden="true" className="pointer-events-none absolute inset-0 rounded-[var(--radius-panel)]">
      <div style={HALO_STYLE} className={lit ? 'opacity-90' : 'opacity-40'} />
      <div style={RING_STYLE} className={lit ? 'opacity-100' : 'opacity-55'} />
    </div>
  )
}
