import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { SegmentState, type TranscriptSegment, TranslationState } from '@/shared/types'
import { StreamView } from './StreamView'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

function segment(id: string): TranscriptSegment {
  return {
    id,
    revision: 1,
    startMs: 0,
    endMs: 100,
    text: id,
    state: SegmentState.Final,
    translationState: TranslationState.Pending,
  }
}

describe('StreamView', () => {
  let container: HTMLDivElement
  let root: Root

  async function render(segments: TranscriptSegment[], fontSize = 22, error: string | null = null) {
    await act(async () => {
      root.render(
        <StreamView
          segments={segments}
          fontSize={fontSize}
          fontColor="#ffffff"
          opacity={60}
          bgColor="#101018"
          glow={false}
          error={error}
        />,
      )
    })
    const scroll = container.querySelector<HTMLDivElement>('.overflow-y-auto')
    if (!scroll) throw new Error('字幕滚动区域未找到')
    return scroll
  }

  beforeEach(() => {
    container = document.createElement('div')
    document.body.append(container)
    root = createRoot(container)
  })

  afterEach(async () => {
    await act(async () => root.unmount())
    container.remove()
  })

  it('follows translation completion in an earlier segment while the tail stays unchanged', async () => {
    const first = segment('first')
    const tail = segment('tail')
    const scroll = await render([first, tail])
    Object.defineProperty(scroll, 'scrollHeight', { value: 180 })
    await render([{ ...first, revision: 2, translation: '完成翻译', translationState: TranslationState.Done }, tail])
    expect(scroll.scrollTop).toBe(180)
  })

  it('keeps the latest text visible after changing font size', async () => {
    const segments = [segment('first')]
    const scroll = await render(segments)
    Object.defineProperty(scroll, 'scrollHeight', { value: 180 })
    await render(segments, 30)
    expect(scroll.scrollTop).toBe(180)
  })

  it('shows stream failures in the overlay', async () => {
    await render([], 22, '字幕订阅失败：监听不可用')
    expect(container.querySelector('[role="alert"]')?.textContent).toContain('字幕订阅失败')
  })
})
