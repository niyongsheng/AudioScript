import { memo, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useI18n } from '@/shared/i18n'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'
import { FlowingBorder } from './FlowingBorder'

const SOURCE_SCALE = 0.8

const SOURCE_OPACITY = 0.7

const PARTIAL_OPACITY = 0.55

const TYPEWRITER_MIN_DELTA = 8

const TYPEWRITER_MS = 260

const STICK_BOTTOM_PX = 24

interface Props {
  segments: TranscriptSegment[]
  fontSize: number
  fontColor: string
  opacity: number
  bgColor: string
  glow: boolean
  glowSpeed: number
  glowWidth: number
  streaming: boolean
}

export function StreamView({
  segments,
  fontSize,
  fontColor,
  opacity,
  bgColor,
  glow,
  glowSpeed,
  glowWidth,
  streaming,
}: Props) {
  const { t } = useI18n()
  const scrollRef = useRef<HTMLDivElement>(null)
  const contentRef = useRef<HTMLDivElement>(null)
  const stickRef = useRef(true)

  const scrollToBottom = useCallback(() => {
    const el = scrollRef.current
    if (el) el.scrollTop = el.scrollHeight
  }, [])

  useLayoutEffect(() => {
    const el = scrollRef.current
    const content = contentRef.current
    if (!el || !content) return
    const observer = new ResizeObserver(() => {
      if (stickRef.current) el.scrollTop = el.scrollHeight
    })
    observer.observe(content)
    return () => observer.disconnect()
  }, [])

  const tail = segments.at(-1)
  const followKey = tail ? `${tail.id}:${tail.revision}` : ''

  // biome-ignore lint/correctness/useExhaustiveDependencies: 意图是 followKey 变化时滚到底，体内无需读取
  useLayoutEffect(() => {
    stickRef.current = true
    scrollToBottom()
  }, [followKey, scrollToBottom])

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
        onScroll={() => {
          const el = scrollRef.current
          if (!el) return
          stickRef.current = el.scrollHeight - el.scrollTop - el.clientHeight <= STICK_BOTTOM_PX
        }}
      >
        <div ref={contentRef}>
          {segments.length === 0 ? (
            <p className="m-0 text-[0.8em]" style={{ color: fontColor, opacity: 0.45 }}>
              {t('overlayWindow.waiting')}
            </p>
          ) : (
            segments.map((s, i) => <BilingualSegment key={s.id} seg={s} active={i === segments.length - 1} />)
          )}
        </div>
      </div>

      {glow && <FlowingBorder streaming={streaming} speed={glowSpeed} width={glowWidth} />}
    </div>
  )
}

const BilingualSegment = memo(function BilingualSegment({ seg, active }: { seg: TranscriptSegment; active: boolean }) {
  const { t } = useI18n()
  const isPartial = seg.state === SegmentState.Partial
  const [shown, setShown] = useState(seg.text)
  const shownRef = useRef(seg.text)
  const rafRef = useRef<number | null>(null)

  useEffect(() => {
    const full = seg.text
    const current = shownRef.current
    const reveal = isPartial && full.startsWith(current) && full.length - current.length > TYPEWRITER_MIN_DELTA

    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current)
      rafRef.current = null
    }

    if (!reveal) {
      shownRef.current = full
      setShown(full)
      return
    }

    const from = current.length
    const started = performance.now()
    const tick = (now: number) => {
      const progress = Math.min(1, (now - started) / TYPEWRITER_MS)
      const next = full.slice(0, from + Math.floor((full.length - from) * progress))
      if (next !== shownRef.current) {
        shownRef.current = next
        setShown(next)
      }
      if (progress < 1) {
        rafRef.current = requestAnimationFrame(tick)
      } else {
        rafRef.current = null
        shownRef.current = full
        setShown(full)
      }
    }
    rafRef.current = requestAnimationFrame(tick)

    return () => {
      if (rafRef.current !== null) {
        cancelAnimationFrame(rafRef.current)
        rafRef.current = null
      }
    }
  }, [seg.text, isPartial])

  const text = isPartial ? shown : seg.text

  return (
    <div
      className={`py-[0.2em] ${seg.state === SegmentState.Edited ? 'border-[var(--color-accent)] border-s-2 ps-2' : ''}`}
    >
      <p
        className="m-0"
        dir="auto"
        style={{
          fontSize: `${SOURCE_SCALE}em`,
          opacity: isPartial ? PARTIAL_OPACITY : SOURCE_OPACITY,
        }}
      >
        {text}
        {isPartial && active && <Caret />}
      </p>

      {seg.translationState === TranslationState.Done && seg.translation && (
        <p className="m-0" dir="auto">
          {seg.translation}
        </p>
      )}
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
    <span className="ms-0.5 inline-block h-[1em] w-[2px] translate-y-[0.15em] animate-pulse bg-current align-baseline" />
  )
}

function hexToRgba(hex: string, alpha: number): string {
  const m = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
  if (!m) return `rgba(16, 16, 24, ${alpha})`
  const n = Number.parseInt(m[1], 16)
  return `rgba(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255}, ${alpha})`
}
