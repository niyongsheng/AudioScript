import { act, StrictMode } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SegmentState, type TranscriptSegment, TranslationState } from './types'
import { useTranscriptStream } from './useTranscriptStream'

const ipc = vi.hoisted(() => ({ on: vi.fn(), snapshot: vi.fn() }))

vi.mock('./ipc', () => ({
  EVENT: { segments: 'transcript://segments' },
  on: ipc.on,
  sessionSnapshot: ipc.snapshot,
  formatCmdError: (error: unknown) => String(error),
}))

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

function segment(id: string, revision = 1): TranscriptSegment {
  return {
    id,
    revision,
    startMs: 0,
    endMs: 100,
    text: `${id}:${revision}`,
    state: SegmentState.Final,
    translationState: TranslationState.Idle,
  }
}

describe('useTranscriptStream', () => {
  let container: HTMLDivElement
  let root: Root
  let current: ReturnType<typeof useTranscriptStream>
  let registration: ReturnType<typeof deferred<() => void>>
  let snapshot: ReturnType<typeof deferred<TranscriptSegment[]>>
  let broadcast: (delta: TranscriptSegment[]) => void
  let unlisten = vi.fn<() => void>()

  function Harness() {
    current = useTranscriptStream()
    return <p role="status">{current.error}</p>
  }

  async function render() {
    await act(async () => root.render(<Harness />))
  }

  async function subscribe() {
    await act(async () => registration.resolve(unlisten))
  }

  beforeEach(() => {
    ipc.on.mockReset()
    ipc.snapshot.mockReset()
    registration = deferred<() => void>()
    snapshot = deferred<TranscriptSegment[]>()
    unlisten = vi.fn<() => void>()
    ipc.on.mockImplementation((_event, handler) => {
      broadcast = handler
      return registration.promise
    })
    ipc.snapshot.mockReturnValue(snapshot.promise)
    container = document.createElement('div')
    document.body.append(container)
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
    container.remove()
  })

  it('waits until the listener is registered before requesting the snapshot', async () => {
    await render()
    expect(ipc.on).toHaveBeenCalledOnce()
    expect(ipc.snapshot).not.toHaveBeenCalled()

    await subscribe()
    expect(ipc.snapshot).toHaveBeenCalledOnce()
  })

  it('keeps snapshot history and replays buffered deltas by revision', async () => {
    await render()
    await subscribe()
    const history = segment('history')
    const old = segment('updating')
    const latest = segment('updating', 3)
    const fresh = segment('new')

    await act(async () => {
      broadcast([latest, fresh])
      broadcast([segment('updating', 2)])
    })
    expect(current.segments).toEqual([latest, fresh])

    await act(async () => snapshot.resolve([history, old]))
    expect(current.segments).toEqual([history, latest, fresh])

    await act(async () => broadcast([old]))
    expect(current.segments).toEqual([history, latest, fresh])
  })

  it('keeps a newer snapshot revision when an older delta was buffered', async () => {
    await render()
    await subscribe()
    await act(async () => broadcast([segment('updating', 2)]))
    const latest = segment('updating', 3)
    await act(async () => snapshot.resolve([latest]))
    expect(current.segments).toEqual([latest])
  })

  it('does not resurrect a stale snapshot after a clear event', async () => {
    await render()
    await subscribe()
    const fresh = segment('after-clear')
    await act(async () => {
      broadcast([segment('before-clear')])
      broadcast([])
      broadcast([fresh])
      snapshot.resolve([segment('old-history')])
    })
    expect(current.segments).toEqual([fresh])
  })

  it('does not resurrect a stale snapshot after a local clear', async () => {
    await render()
    await subscribe()
    await act(async () => broadcast([segment('before-clear')]))
    await act(async () => current.clear())
    await act(async () => snapshot.resolve([segment('old-history')]))
    expect(current.segments).toEqual([])
  })

  it('accepts a fresh snapshot when a clear happened before snapshot loading started', async () => {
    await render()
    await act(async () => broadcast([]))
    await subscribe()
    const fresh = segment('after-clear')
    await act(async () => snapshot.resolve([fresh]))
    expect(current.segments).toEqual([fresh])
  })

  it('discards pre-clear buffered deltas when a local clear precedes snapshot loading', async () => {
    await render()
    await act(async () => broadcast([segment('before-clear')]))
    await act(async () => current.clear())
    await subscribe()
    const fresh = segment('after-clear')
    await act(async () => snapshot.resolve([fresh]))
    expect(current.segments).toEqual([fresh])
  })

  it('cleans up registration that completes after unmount without loading a snapshot', async () => {
    await render()
    await act(async () => root.unmount())
    await subscribe()
    expect(unlisten).toHaveBeenCalledOnce()
    expect(ipc.snapshot).not.toHaveBeenCalled()
  })

  it('unsubscribes and ignores late events and snapshot responses after unmount', async () => {
    await render()
    await subscribe()
    await act(async () => broadcast([segment('visible')]))
    const beforeUnmount = current
    await act(async () => root.unmount())
    await act(async () => {
      broadcast([segment('late')])
      snapshot.resolve([segment('late-snapshot')])
    })
    expect(unlisten).toHaveBeenCalledOnce()
    expect(current).toBe(beforeUnmount)
  })

  it('exposes subscription failure instead of displaying an unexplained empty transcript', async () => {
    await render()
    await act(async () => registration.reject(new Error('监听不可用')))
    expect(current.error).toContain('字幕订阅失败')
    expect(container.textContent).toContain('监听不可用')
    expect(ipc.snapshot).not.toHaveBeenCalled()
  })

  it('exposes snapshot failure and continues accepting live deltas', async () => {
    await render()
    await subscribe()
    const first = segment('first')
    const second = segment('second')
    await act(async () => {
      broadcast([first])
      snapshot.reject(new Error('读取不可用'))
    })
    expect(current.error).toContain('读取转写记录失败')
    expect(container.textContent).toContain('读取不可用')
    await act(async () => broadcast([second]))
    expect(current.segments).toEqual([first, second])
  })

  it('releases the first registration during Strict Mode effect replay', async () => {
    const initialRegistration = deferred<() => void>()
    const initialUnlisten = vi.fn()
    ipc.on.mockImplementationOnce(() => initialRegistration.promise)
    await act(async () => {
      root.render(
        <StrictMode>
          <Harness />
        </StrictMode>,
      )
    })
    await act(async () => initialRegistration.resolve(initialUnlisten))
    expect(initialUnlisten).toHaveBeenCalledOnce()
    expect(ipc.snapshot).not.toHaveBeenCalled()
    await subscribe()
    expect(ipc.snapshot).toHaveBeenCalledOnce()
  })
})
