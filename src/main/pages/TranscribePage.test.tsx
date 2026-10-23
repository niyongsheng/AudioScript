import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'
import { TranscribePage } from './TranscribePage'

const mocks = vi.hoisted(() => ({
  clearAll: vi.fn(),
  localClear: vi.fn(),
  setError: vi.fn(),
  segments: [] as TranscriptSegment[],
  streamError: null as string | null,
}))

vi.mock('@/main/store', () => ({
  useStore: () => ({ status: 'ready', clearAll: mocks.clearAll, setError: mocks.setError }),
}))

vi.mock('@/shared/useTranscriptStream', () => ({
  useTranscriptStream: () => ({ segments: mocks.segments, clear: mocks.localClear, error: mocks.streamError }),
}))

vi.mock('@/shared/ipc', async () => ({
  ...(await vi.importActual<typeof import('@/shared/ipc')>('@/shared/ipc')),
  revealPath: vi.fn(),
  transcriptExport: vi.fn(),
}))

vi.mock('@tauri-apps/plugin-dialog', () => ({ save: vi.fn() }))

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

describe('TranscribePage clear and stream failures', () => {
  let container: HTMLDivElement
  let root: Root

  async function render() {
    await act(async () => root.render(<TranscribePage />))
  }

  function clearButton() {
    const button = Array.from(container.querySelectorAll('button')).find((item) => item.textContent?.trim() === '清空')
    if (!button) throw new Error('清空按钮未找到')
    return button
  }

  beforeEach(() => {
    mocks.clearAll.mockReset()
    mocks.localClear.mockReset()
    mocks.setError.mockReset()
    mocks.streamError = null
    mocks.segments = [
      {
        id: 'saved',
        startMs: 0,
        endMs: 100,
        text: '需要保留的转写',
        state: SegmentState.Final,
        translationState: TranslationState.Idle,
        revision: 1,
      },
    ]
    container = document.createElement('div')
    document.body.append(container)
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
    container.remove()
  })

  it('reports a backend clear rejection and preserves the local transcript', async () => {
    mocks.clearAll.mockRejectedValue(new Error('上一次会话仍在收尾，请稍后重试'))
    await render()
    await act(async () => clearButton().click())

    expect(mocks.clearAll).toHaveBeenCalledOnce()
    expect(mocks.localClear).not.toHaveBeenCalled()
    expect(mocks.setError).toHaveBeenCalledWith(expect.stringContaining('上一次会话仍在收尾'))
    expect(container.textContent).toContain('需要保留的转写')
  })

  it('clears local state only after the backend confirms success', async () => {
    let finishClear!: () => void
    mocks.clearAll.mockReturnValue(
      new Promise<void>((resolve) => {
        finishClear = resolve
      }),
    )
    await render()
    await act(async () => clearButton().click())
    expect(mocks.clearAll).toHaveBeenCalledOnce()
    expect(mocks.localClear).not.toHaveBeenCalled()

    await act(async () => finishClear())
    expect(mocks.localClear).toHaveBeenCalledOnce()
    expect(mocks.setError).not.toHaveBeenCalled()
  })

  it('shows stream synchronization errors alongside existing transcript content', async () => {
    mocks.streamError = '字幕订阅失败：监听不可用'
    await render()

    const alert = container.querySelector('[role="alert"]')
    expect(alert?.textContent).toContain('转写内容同步失败')
    expect(alert?.textContent).toContain('字幕订阅失败：监听不可用')
    expect(container.textContent).toContain('需要保留的转写')
  })
})
