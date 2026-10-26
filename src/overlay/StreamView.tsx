import { type CSSProperties, memo, useLayoutEffect, useRef } from 'react'
import { useI18n } from '@/shared/i18n'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'

const SOURCE_SCALE = 0.8

const SOURCE_OPACITY = 0.7

const PARTIAL_OPACITY = 0.55

interface Props {
  segments: TranscriptSegment[]
  fontSize: number
  fontColor: string
  opacity: number
  bgColor: string
  glow: boolean
}

export function StreamView({ segments, fontSize, fontColor, opacity, bgColor, glow }: Props) {
  const { t } = useI18n()
  const scrollRef = useRef<HTMLDivElement>(null)

  const tail = segments.at(-1)
  const followKey = tail ? `${tail.id}:${tail.revision}` : ''

  // biome-ignore lint/correctness/useExhaustiveDependencies: 意图是 followKey 变化时滚到底，体内无需读取
  useLayoutEffect(() => {
    const el = scrollRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [followKey])

  const bg = hexToRgba(bgColor, opacity / 100)

  return (
    <div className="relative flex h-full w-full flex-col">
      <div
        ref={scrollRef}
        className="relative min-h-0 w-full flex-1 overflow-y-auto rounded-[var(--radius-panel)] px-4 py-2"
        style={{
          background: bg,
          fontSize: `${fontSize}px`,
          color: fontColor,
          lineHeight: 1.35,
          scrollbarWidth: 'none',
        }}
      >
        {segments.length === 0 ? (
          <p className="m-0 text-[0.8em]" style={{ color: fontColor, opacity: 0.45 }}>
            {t('overlayWindow.waiting')}
          </p>
        ) : (
          segments.map((s) => <BilingualSegment key={s.id} seg={s} />)
        )}
      </div>

      {glow && (
        <div
          aria-hidden="true"
          className="pointer-events-none absolute inset-0 rounded-[var(--radius-panel)]"
          style={rainbowRing(3)}
        />
      )}
    </div>
  )
}

const RAINBOW_GRADIENT = 'linear-gradient(90deg, #b06ab3 0%, #4facfe 25%, #47cf73 50%, #ffc371 75%, #ff5f6d 100%)'

function rainbowRing(padding: number): CSSProperties {
  const maskImage = 'linear-gradient(#000 0 0), linear-gradient(#000 0 0)'
  return {
    background: RAINBOW_GRADIENT,
    padding,
    maskImage,
    maskClip: 'content-box, border-box',
    maskComposite: 'exclude',
    WebkitMaskImage: maskImage,
    WebkitMaskClip: 'content-box, border-box',
    WebkitMaskComposite: 'xor',
  }
}

const BilingualSegment = memo(function BilingualSegment({ seg }: { seg: TranscriptSegment }) {
  const { t } = useI18n()
  const isPartial = seg.state === SegmentState.Partial

  return (
    <div
      className={`py-[0.2em] ${seg.state === SegmentState.Edited ? 'border-[var(--color-accent)] border-l-2 pl-2' : ''}`}
    >
      <p
        className="m-0"
        style={{
          fontSize: `${SOURCE_SCALE}em`,
          opacity: isPartial ? PARTIAL_OPACITY : SOURCE_OPACITY,
        }}
      >
        {seg.text}
        {isPartial && <Caret />}
      </p>

      {seg.translationState === TranslationState.Done && seg.translation && <p className="m-0">{seg.translation}</p>}
      {seg.translationState === TranslationState.Failed && (
        <p className="m-0 opacity-40" style={{ fontSize: `${SOURCE_SCALE}em` }}>
          <span className="italic">{t('overlayWindow.translationFailed')}</span>
        </p>
      )}
    </div>
  )
})

function Caret() {
  return (
    <span className="ml-0.5 inline-block h-[1em] w-[2px] translate-y-[0.15em] animate-pulse bg-current align-baseline" />
  )
}

function hexToRgba(hex: string, alpha: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return `rgba(16, 16, 24, ${alpha})`
  const n = Number.parseInt(m[1], 16)
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`
}
