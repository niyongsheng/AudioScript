import { memo, useEffect, useLayoutEffect, useRef } from 'react'
import { useI18n } from '@/shared/i18n'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'
import { cn } from './cn'

interface Props {
  segments: TranscriptSegment[]
}

export function SegmentList({ segments }: Props) {
  const ref = useRef<HTMLOListElement>(null)
  const atBottom = useRef(true)

  useEffect(() => {
    const el = ref.current?.parentElement
    if (!el) return
    const onScroll = () => {
      atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24
    }
    el.addEventListener('scroll', onScroll, { passive: true })
    return () => el.removeEventListener('scroll', onScroll)
  }, [])

  // biome-ignore lint/correctness/useExhaustiveDependencies: 意图是段落数变化时滚到底，体内无需读取
  useLayoutEffect(() => {
    const el = ref.current?.parentElement
    if (el && atBottom.current) el.scrollTop = el.scrollHeight
  }, [segments.length])

  return (
    <ol ref={ref} className="space-y-3">
      {segments.map((s) => (
        <SegmentItem key={s.id} seg={s} />
      ))}
    </ol>
  )
}

const SegmentItem = memo(function SegmentItem({ seg }: { seg: TranscriptSegment }) {
  const { t } = useI18n()
  return (
    <li className={cn(seg.state === SegmentState.Edited && 'border-[var(--color-accent)] border-s-2 ps-3')}>
      <p className="m-0 text-[0.78em] leading-relaxed" style={{ color: 'var(--color-segment-partial)' }}>
        {seg.text}
        {seg.state === SegmentState.Partial && (
          <span className="ms-0.5 inline-block h-[1em] w-[2px] translate-y-[0.15em] animate-pulse bg-current align-baseline" />
        )}
      </p>

      {seg.translationState === TranslationState.Done && seg.translation && (
        <p className="m-0 text-[0.95em] text-[var(--color-segment-final)] leading-relaxed">{seg.translation}</p>
      )}

      {seg.translationState === TranslationState.Failed && (
        <p className="m-0 text-[0.78em] text-[var(--color-warn)] italic opacity-70">
          {t('overlayWindow.translationFailed')}
        </p>
      )}
    </li>
  )
})
