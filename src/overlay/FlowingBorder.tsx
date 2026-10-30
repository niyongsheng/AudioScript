import { type CSSProperties, useEffect, useMemo, useRef } from 'react'
import { GLOW_RING_PALETTE } from '@/shared/glow'
import { MAX_GLOW_SPEED } from '@/shared/types'

const MAX_FLOW_DEG_PER_SEC = 120

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

export function FlowingBorder({ streaming, speed, width }: { streaming: boolean; speed: number; width: number }) {
  const haloStyle = useMemo(() => ringStyle(width + 1, width + 2), [width])
  const ringStyleValue = useMemo(() => ringStyle(width, 0), [width])
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
      <div style={haloStyle} className={lit ? 'opacity-90' : 'opacity-40'} />
      <div style={ringStyleValue} className={lit ? 'opacity-100' : 'opacity-55'} />
    </div>
  )
}
