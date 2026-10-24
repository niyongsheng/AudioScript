import { memo, useEffect, useLayoutEffect, useRef } from 'react'
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

  // biome-ignore lint/correctness/useExhaustiveDependencies: 同段原文增长和译文回填也会改变内容高度
  useLayoutEffect(() => {
    const el = ref.current?.parentElement
    if (el && atBottom.current) el.scrollTop = el.scrollHeight
  }, [segments])

  return (
    <ol ref={ref} className="space-y-3">
      {segments.map((s) => (
        <SegmentItem key={s.id} seg={s} />
      ))}
    </ol>
  )
}

const SegmentItem = memo(function SegmentItem({ seg }: { seg: TranscriptSegment }) {
  return (
    <li className={cn(seg.state === SegmentState.Edited && 'border-[var(--color-accent)] border-l-2 pl-3')}>
      <p className="m-0 text-[0.78em] leading-relaxed" style={{ color: 'var(--color-segment-partial)' }}>
        {seg.text}
        {seg.state === SegmentState.Partial && (
          <span className="ml-0.5 inline-block h-[1em] w-[2px] translate-y-[0.15em] animate-pulse bg-current align-baseline" />
        )}
      </p>

      {seg.translationState === TranslationState.Done && seg.translation && (
        <p className="m-0 text-[0.95em] text-[var(--color-segment-final)] leading-relaxed">{seg.translation}</p>
      )}

      {seg.translationState === TranslationState.Failed && (
        <p className="m-0 text-[0.78em] text-[var(--color-warn)] italic opacity-70">翻译失败</p>
      )}
    </li>
  )
})
