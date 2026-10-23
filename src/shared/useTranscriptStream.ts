import { useCallback, useEffect, useRef, useState } from 'react'
import { EVENT, formatCmdError, on, sessionSnapshot } from './ipc'
import { mergeDelta } from './transcript'
import type { TranscriptSegment } from './types'

export function useTranscriptStream(): {
  segments: TranscriptSegment[]
  clear: () => void
  error: string | null
} {
  const [segments, setSegments] = useState<TranscriptSegment[]>([])
  const [error, setError] = useState<string | null>(null)
  const clearVersion = useRef(0)

  const clear = useCallback(() => {
    clearVersion.current += 1
    setSegments([])
  }, [])

  useEffect(() => {
    let disposed = false
    let snapshotPending = true
    let buffered: TranscriptSegment[] = []
    let bufferVersion = clearVersion.current
    let unlisten: (() => void) | null = null
    setError(null)

    async function connect() {
      try {
        const stop = await on<TranscriptSegment[]>(EVENT.segments, (delta) => {
          if (disposed) return
          if (delta.length === 0) {
            buffered = []
            clear()
            return
          }
          if (snapshotPending) {
            if (bufferVersion !== clearVersion.current) buffered = []
            bufferVersion = clearVersion.current
            buffered = mergeDelta(buffered, delta)
          }
          setSegments((prev) => mergeDelta(prev, delta))
        })
        if (disposed) {
          stop()
          return
        }
        unlisten = stop
      } catch (e) {
        if (!disposed) setError(`字幕订阅失败：${formatCmdError(e)}`)
        return
      }

      const version = clearVersion.current
      try {
        const snap = await sessionSnapshot()
        if (!disposed && version === clearVersion.current) {
          const replay = bufferVersion === version ? buffered : []
          setSegments(mergeDelta(snap, replay))
        }
      } catch (e) {
        if (!disposed) setError(`读取转写记录失败：${formatCmdError(e)}`)
      } finally {
        snapshotPending = false
        buffered = []
      }
    }

    void connect()

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [clear])

  return { segments, clear, error }
}
