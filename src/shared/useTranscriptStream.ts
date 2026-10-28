import { useState } from 'react'
import { EVENT, sessionSnapshot } from './ipc'
import { mergeDelta } from './transcript'
import type { TranscriptSegment } from './types'
import { useBroadcastSnapshot } from './useBroadcastSnapshot'

export function useTranscriptStream(): {
  segments: TranscriptSegment[]
  clear: () => void
} {
  const [segments, setSegments] = useState<TranscriptSegment[]>([])

  useBroadcastSnapshot<TranscriptSegment[]>(EVENT.segments, sessionSnapshot, (delta) => {
    setSegments((prev) => (delta.length === 0 ? [] : mergeDelta(prev, delta)))
  })

  return { segments, clear: () => setSegments([]) }
}
