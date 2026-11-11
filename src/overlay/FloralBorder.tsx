import { getCurrentWindow } from '@tauri-apps/api/window'
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { call, EVENT, on } from '@/shared/ipc'
import { isLightPanel, runFloralBorder } from './florilegium'

const BLOOM_MARGIN = 56

interface ActiveBloom {
  id: number
  width: number
  height: number
  margin: number
}

export interface Bloom extends ActiveBloom {
  seed: number
}

export function useEggBloom() {
  const [bloom, setBloom] = useState<Bloom | null>(null)
  const nextId = useRef(0)
  const active = useRef<ActiveBloom | null>(null)

  const end = useCallback(() => {
    const current = active.current
    if (!current) return
    active.current = null
    call('overlay_bloom_end')
      .catch((e) => console.error('彩蛋花边收尾失败', e))
      .finally(() => setBloom((latest) => (latest && latest.id === current.id ? null : latest)))
  }, [])

  useEffect(() => {
    const win = getCurrentWindow()
    const unlisteners: (() => void)[] = []
    let disposed = false

    const track = (subscription: Promise<() => void>, label: string) => {
      subscription
        .then((fn) => {
          if (disposed) fn()
          else unlisteners.push(fn)
        })
        .catch((e) => console.error(label, e))
    }

    const start = async () => {
      if (!(await win.isVisible())) return
      const existing = active.current
      const width = existing?.width ?? window.innerWidth
      const height = existing?.height ?? window.innerHeight
      const margin = existing?.margin ?? BLOOM_MARGIN
      nextId.current += 1
      const id = nextId.current
      active.current = { id, width, height, margin }
      setBloom({ id, width, height, margin, seed: (Math.random() * 0x7fffffff) | 0 })
      try {
        await call('overlay_bloom_begin', { margin })
        if (disposed || !(await win.isVisible())) end()
      } catch (e) {
        console.error('彩蛋花边启动失败', e)
        end()
      }
    }

    track(
      on(EVENT.eggBloom, () => void start()),
      '彩蛋花边监听失败',
    )
    track(
      on<boolean>(EVENT.overlayVisibility, (visible) => {
        if (!visible) end()
      }),
      '彩蛋花边可见性监听失败',
    )

    return () => {
      disposed = true
      for (const fn of unlisteners) fn()
      if (active.current) {
        active.current = null
        call('overlay_bloom_end').catch((e) => console.error('彩蛋花边收尾失败', e))
      }
    }
  }, [end])

  const finish = useCallback(
    (id: number) => {
      if (active.current?.id === id) end()
    },
    [end],
  )

  return { bloom, finish }
}

export function FloralBorder({ bloom, bgColor, onDone }: { bloom: Bloom; bgColor: string; onDone: () => void }) {
  return (
    <BloomCanvas key={bloom.id} seed={bloom.seed} margin={bloom.margin} light={isLightPanel(bgColor)} onDone={onDone} />
  )
}

function BloomCanvas({
  seed,
  margin,
  light,
  onDone,
}: {
  seed: number
  margin: number
  light: boolean
  onDone: () => void
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const doneRef = useRef(onDone)

  useLayoutEffect(() => {
    doneRef.current = onDone
  })

  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas) return
    return runFloralBorder(canvas, {
      seed,
      margin,
      light,
      reducedMotion: window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false,
      onDone: () => doneRef.current(),
    })
  }, [seed, margin, light])

  return <canvas ref={canvasRef} className="pointer-events-none absolute inset-0 z-[5] h-full w-full" />
}
