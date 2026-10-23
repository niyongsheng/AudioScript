import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'
import { SegmentList } from './SegmentList'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

function segment(patch: Partial<TranscriptSegment> = {}): TranscriptSegment {
  return {
    id: 'speaking',
    revision: 1,
    startMs: 0,
    endMs: 100,
    text: '正在讲话',
    state: SegmentState.Partial,
    translationState: TranslationState.Idle,
    ...patch,
  }
}

describe('SegmentList scrolling', () => {
  let container: HTMLDivElement
  let root: Root
  let contentHeight: number

  async function render(segments: TranscriptSegment[]) {
    await act(async () => root.render(<SegmentList segments={segments} />))
  }

  beforeEach(() => {
    contentHeight = 100
    container = document.createElement('div')
    Object.defineProperties(container, {
      scrollHeight: { get: () => contentHeight },
      clientHeight: { get: () => 40 },
    })
    document.body.append(container)
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
    container.remove()
    vi.restoreAllMocks()
  })

  it('follows growing partial text even when the segment count stays the same', async () => {
    await render([segment()])
    expect(container.scrollTop).toBe(100)
    contentHeight = 180
    await render([segment({ text: '正在讲话，内容继续增长并产生换行', revision: 2 })])
    expect(container.scrollTop).toBe(180)
  })

  it('follows translation completion when it adds height to an existing segment', async () => {
    await render([segment({ state: SegmentState.Final, translationState: TranslationState.Pending })])
    contentHeight = 160
    await render([
      segment({
        revision: 2,
        state: SegmentState.Final,
        translation: 'The speaker is talking.',
        translationState: TranslationState.Done,
      }),
    ])
    expect(container.scrollTop).toBe(160)
  })

  it('preserves the reader position when they have scrolled away from the bottom', async () => {
    await render([segment()])
    await act(async () => {
      container.scrollTop = 0
      container.dispatchEvent(new Event('scroll'))
    })
    contentHeight = 180
    await render([segment({ text: '新内容', revision: 2 })])
    expect(container.scrollTop).toBe(0)
  })

  it('resumes following after the reader scrolls back to the bottom', async () => {
    await render([segment()])
    await act(async () => {
      container.scrollTop = 0
      container.dispatchEvent(new Event('scroll'))
      container.scrollTop = 60
      container.dispatchEvent(new Event('scroll'))
    })
    contentHeight = 180
    await render([segment({ revision: 2 })])
    expect(container.scrollTop).toBe(180)
  })
})
