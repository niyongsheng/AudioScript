import { useEffect, useLayoutEffect, useRef } from 'react'
import { on } from './ipc'

export function useBroadcastSnapshot<T>(
  event: string,
  fetchSnapshot: () => Promise<T>,
  apply: (value: T) => void,
): void {
  const latest = useRef({ fetchSnapshot, apply })
  useLayoutEffect(() => {
    latest.current = { fetchSnapshot, apply }
  })

  useEffect(() => {
    let disposed = false
    let gotBroadcast = false
    let unlisten: (() => void) | null = null

    on<T>(event, (value) => {
      gotBroadcast = true
      if (!disposed) latest.current.apply(value)
    })
      .then((fn) => {
        if (disposed) fn()
        else unlisten = fn
      })
      .catch(() => {})

    latest.current
      .fetchSnapshot()
      .then((value) => {
        if (!disposed && !gotBroadcast) latest.current.apply(value)
      })
      .catch(() => {})

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [event])
}
